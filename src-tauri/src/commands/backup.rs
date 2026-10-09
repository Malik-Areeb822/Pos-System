// src-tauri/src/commands/backup.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_admin;
use crate::database::{Db, DbPool};
use crate::error::AppError;
use std::path::{Path, PathBuf};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::fs;

/// Expected core tables a valid MoonPipe database must contain.
/// `app_settings` (migration 016) is deliberately NOT listed: backups older
/// than 2026-10-06 predate it, and `import_database` re-creates the table on
/// restore, so rejecting those backups would be pure friction (M4).
const REQUIRED_TABLES: &[&str] = &[
    "users",
    "products",
    "customers",
    "invoices",
    "invoice_items",
    "returns",
    "_sqlx_migrations",
];

fn backups_dir(config: &crate::config::Config) -> PathBuf {
    config.app_data_dir.join("backups")
}

/// Millisecond-resolution stamp: a user export and a retention snapshot can
/// otherwise land in the same second and silently overwrite each other (L2).
fn timestamp() -> String {
    chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string()
}

/// Escape a path for use as an SQLite string literal (VACUUM INTO).
fn sql_literal(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "''"))
}

/// Delete stale WAL side files next to `db_path`. Every pool on that file must
/// already be closed: a leftover `-wal`/`-shm` belonging to the *previous*
/// database would otherwise be replayed into the freshly copied one.
/// A failure to remove one is fatal (M1) — swallowing it is how split-brain
/// corruption starts. Absent files are fine.
async fn remove_stale_wal_files(db_path: &Path) -> Result<(), String> {
    let base = db_path.to_string_lossy().to_string();
    for suffix in ["-wal", "-shm"] {
        let target = format!("{base}{suffix}");
        if let Err(e) = fs::remove_file(&target).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("could not remove stale {suffix} file ({e})"));
            }
        }
    }
    Ok(())
}

/// H1 — refuse to restore a file onto the live database it was copied from.
/// The Settings picker accepts `.db`, and a hand-copied `moonpipe.db` is the
/// one realistic way to pick the live file; restoring it over itself silently
/// discards everything not yet checkpointed. Canonicalize both sides (resolves
/// case and 8.3 short names on Windows) and compare case-insensitively.
fn ensure_not_live_db(src: &Path, live: &Path) -> Result<(), AppError> {
    let src_final = std::fs::canonicalize(src).map_err(|e| {
        AppError::Validation(format!("Could not resolve the selected file path ({e})"))
    })?;
    let live_final = match std::fs::canonicalize(live) {
        Ok(p) => p,
        Err(_) => live.to_path_buf(), // no live file yet — nothing to collide with
    };
    if src_final
        .to_string_lossy()
        .eq_ignore_ascii_case(&live_final.to_string_lossy())
    {
        return Err(AppError::Validation(
            "That file is the live database (moonpipe.db). Restoring it over itself would \
             discard recent sales — pick a backup from the backups folder instead."
                .into(),
        ));
    }
    Ok(())
}

/// H2 — fold any WAL sidecars into the staged file and force a rollback
/// journal so the stage becomes a single self-contained SQLite file. Without
/// this, validating a WAL-mode source (which replays its `-wal`) and then
/// swapping without it would silently drop everything that lived only in the
/// sidecar.
async fn prepare_stage(stage: &Path) -> Result<(), AppError> {
    let url = format!(
        "sqlite:{}?mode=rwc",
        stage.to_string_lossy().replace('\\', "/")
    );
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .map_err(|e| {
            AppError::Internal(format!("Selected file could not be opened for restore: {e}"))
        })?;

    // Replay the copied -wal into the main file and truncate it away.
    // (fetch_all: a non-WAL source legitimately returns no rows.)
    sqlx::query_scalar::<_, i64>("PRAGMA wal_checkpoint(TRUNCATE)")
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            AppError::Internal(format!("Selected file is not a valid SQLite database: {e}"))
        })?;

    // Force rollback-journal mode and assert the switch actually took.
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode=DELETE")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            AppError::Internal(format!("Could not normalize the backup journal mode: {e}"))
        })?;
    if !mode.eq_ignore_ascii_case("delete") {
        let _ = pool.close().await;
        return Err(AppError::Internal(format!(
            "Backup is in '{mode}' journal mode and could not be converted to a self-contained file"
        )));
    }
    pool.close().await;
    remove_stale_wal_files(stage).await.map_err(AppError::Internal)?;
    Ok(())
}

/// M3 + M4 — integrity check, required tables (SQL generated *from*
/// `REQUIRED_TABLES` so the constant and the query can never drift again),
/// and a migration-version guard against backups written by a newer app.
async fn validate_stage(stage: &Path) -> Result<(), AppError> {
    let stage_url = format!(
        "sqlite:{}?mode=ro",
        stage.to_string_lossy().replace('\\', "/")
    );
    let probe = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&stage_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            return Err(AppError::Internal(
                "Selected file is not a readable SQLite database".into(),
            ))
        }
    };

    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&probe)
        .await
        .unwrap_or_else(|_| "probe failed".to_string());
    if integrity != "ok" {
        probe.close().await;
        return Err(AppError::Internal(format!(
            "Backup failed integrity check ({integrity})"
        )));
    }

    let placeholders = vec!["?"; REQUIRED_TABLES.len()].join(",");
    let sql = format!(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ({placeholders})"
    );
    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    for table in REQUIRED_TABLES {
        query = query.bind(table);
    }
    let found = query.fetch_one(&probe).await.unwrap_or(0);
    if (found as usize) < REQUIRED_TABLES.len() {
        probe.close().await;
        return Err(AppError::Internal(
            "File does not look like a MoonPipe database".into(),
        ));
    }

    // M3: sqlx validates every applied migration on every `open_pool`
    // (ignore_missing = false), so a backup carrying a migration this build
    // does not embed would make the app exit at next launch. Reject it here,
    // before anything live is touched.
    let embedded_max = sqlx::migrate!("./src/database/migrations")
        .iter()
        .map(|m| m.version)
        .max()
        .unwrap_or(0);
    let stage_version: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
            .fetch_one(&probe)
            .await
            .map_err(|e| {
                AppError::Internal(format!("Could not read the backup's migration version: {e}"))
            })?;
    probe.close().await;
    if stage_version > embedded_max {
        return Err(AppError::Validation(format!(
            "Backup was created by a newer version of Moon Pipe POS \
             (migration {stage_version}) — update the app first"
        )));
    }
    Ok(())
}

/// M2 — put the previous database back after a failed swap. Returns a fresh
/// pool on the restored file; the caller installs it into the holder.
async fn rollback_to_snapshot(db_path: &Path, snapshot: &Path) -> Result<DbPool, String> {
    fs::copy(snapshot, db_path)
        .await
        .map_err(|e| format!("could not copy the pre-restore snapshot back ({e})"))?;
    remove_stale_wal_files(db_path).await?;
    crate::database::open_pool(db_path).await
}

#[tauri::command]
pub async fn export_database(app: AppHandle, db: State<'_, Db>, auth_header: Option<String>) -> Result<String, AppError> {
    require_admin(&app, auth_header).await?;

    let config = crate::config::CONFIG.clone();
    let dir = backups_dir(&config);
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| AppError::Internal(format!("Backup failed: {}", e)))?;
    let backup_path = dir.join(format!("moonpipe_backup_{}.sqlite", timestamp()));
    let _ = fs::remove_file(&backup_path).await; // VACUUM INTO requires the target not to exist

    // Consistent snapshot straight from the live pool: safe against in-flight
    // writes and verifies the database is readable as a side effect.
    let pool = db.pool().await;
    let vacuum = format!("VACUUM INTO {}", sql_literal(&backup_path));
    sqlx::query(&vacuum)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Internal(format!("Backup failed: {}", e)))?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_database(app: AppHandle, db: State<'_, Db>, backup_path: String, auth_header: Option<String>) -> Result<(), AppError> {
    require_admin(&app, auth_header).await?;

    let config = crate::config::CONFIG.clone();
    let src = PathBuf::from(&backup_path);

    // 1-2. Existence check (unchanged).
    if !src.exists() {
        return Err(AppError::NotFound("Backup file not found".into()));
    }

    // 3. Never restore the live database over itself (H1).
    ensure_not_live_db(&src, &config.db_path)?;

    // 4. Stage a copy next to the live DB (same volume) — including the WAL
    //    sidecars when present, so a hand-copied WAL-mode file keeps every
    //    un-checkpointed commit (H2).
    let stage_path = config.app_data_dir.join("restore_staging.sqlite");
    let _ = fs::remove_file(&stage_path).await;
    if fs::copy(&src, &stage_path).await.is_err() {
        return Err(AppError::Internal("Could not read the selected backup file".into()));
    }
    for suffix in ["-wal", "-shm"] {
        let src_side = PathBuf::from(format!("{}{suffix}", src.to_string_lossy()));
        if !src_side.exists() {
            continue;
        }
        let stage_side = PathBuf::from(format!("{}{suffix}", stage_path.to_string_lossy()));
        let _ = fs::remove_file(&stage_side).await;
        if fs::copy(&src_side, &stage_side).await.is_err() {
            let _ = fs::remove_file(&stage_path).await;
            return Err(AppError::Internal("Could not read the selected backup file".into()));
        }
    }

    // 5. Fold the sidecars in and force rollback-journal mode (H2), so the
    //    stage is one self-contained file with no sidecar left to lose.
    if let Err(e) = prepare_stage(&stage_path).await {
        let _ = fs::remove_file(&stage_path).await;
        return Err(e);
    }

    // 6. Validate the prepared copy before touching anything live:
    //    integrity_check + required tables + migration-version guard.
    if let Err(e) = validate_stage(&stage_path).await {
        let _ = fs::remove_file(&stage_path).await;
        return Err(e);
    }

    // 7. Auto-snapshot the current DB first — FATAL on failure (L3). The live
    //    database has not been touched yet, so refusing here is safe, and a
    //    successful snapshot is also the undo file step 10 relies on.
    let snapshot = backups_dir(&config).join(format!("pre_restore_{}.sqlite", timestamp()));
    if let Err(e) = fs::create_dir_all(backups_dir(&config)).await {
        let _ = fs::remove_file(&stage_path).await;
        return Err(AppError::Internal(format!(
            "Restore aborted before touching your data: could not create the backups folder ({e})"
        )));
    }
    {
        let live = db.pool().await;
        let vacuum = format!("VACUUM INTO {}", sql_literal(&snapshot));
        if let Err(e) = sqlx::query(&vacuum).execute(&live).await {
            let _ = fs::remove_file(&stage_path).await;
            return Err(AppError::Internal(format!(
                "Restore aborted before touching your data: could not snapshot current data ({e})"
            )));
        }
    }

    // 8. Swap A: take exclusive access, close the old pool (releases file
    //    locks), then drop side files left by the *previous* database.
    //    Fatal on failure (M1) — but the original file is still intact here,
    //    so revive the old pool and bail out.
    let holder = db.holder();
    let mut guard = holder.write().await;
    let old = guard.clone();
    old.close().await;

    if let Err(e) = remove_stale_wal_files(&config.db_path).await {
        match crate::database::open_pool(&config.db_path).await {
            Ok(revived) => *guard = revived,
            Err(re) => log::error!("Could not revive the previous pool after a failed restore: {re}"),
        }
        let _ = fs::remove_file(&stage_path).await;
        return Err(AppError::Internal(format!("Restore failed: {e}")));
    }

    // 9. Swap B: copy the prepared stage over the live DB (existing revive
    //    branch on copy failure unchanged), then drop side files — fatal,
    //    with rollback (M1 + M2).
    if let Err(e) = fs::copy(&stage_path, &config.db_path).await {
        // Try to bring the old pool back so the app stays usable.
        let _ = remove_stale_wal_files(&config.db_path).await;
        if let Ok(revived) = crate::database::open_pool(&config.db_path).await {
            *guard = revived;
        }
        let _ = fs::remove_file(&stage_path).await;
        return Err(AppError::Internal(format!("Restore failed: {e}")));
    }
    if let Err(e) = remove_stale_wal_files(&config.db_path).await {
        let _ = fs::remove_file(&stage_path).await;
        return match rollback_to_snapshot(&config.db_path, &snapshot).await {
            Ok(pool) => {
                *guard = pool;
                Err(AppError::Internal(format!(
                    "Restore failed and was rolled back to your previous data: {e}"
                )))
            }
            Err(re) => Err(AppError::Internal(format!(
                "Restore failed ({e}) and automatic rollback also failed ({re}). \
                 Your previous data is in {}",
                snapshot.to_string_lossy()
            ))),
        };
    }
    let _ = fs::remove_file(&stage_path).await;

    // 10. Reopen the freshly swapped database.
    match crate::database::open_pool(&config.db_path).await {
        Ok(new_pool) => {
            // Ensure app_settings table and lock row exist (old backups may not have them)
            let _ = sqlx::query(
                "CREATE TABLE IF NOT EXISTS app_settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                )"
            ).execute(&new_pool).await;
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO app_settings (key, value) VALUES ('system_lock', '0')"
            ).execute(&new_pool).await;

            // Bring a restored database up to retention policy too (older
            // backups may contain sales beyond the 12-month window).
            let purge_pool = new_pool.clone();
            // A backup from before the carry-forward work can carry stale
            // balances; recompute them from its own invoice ledger.
            let reconcile_pool = new_pool.clone();
            *guard = new_pool;
            drop(guard);
            let _ = old; // already closed
            crate::events::emit_database_restored(&app).await;
            tauri::async_runtime::spawn(crate::services::retention::run_maintenance(
                app.clone(),
                purge_pool,
            ));
            tauri::async_runtime::spawn(crate::services::reconciliation::run_and_notify(
                app.clone(),
                reconcile_pool,
            ));
            Ok(())
        }
        Err(e) => {
            // The copied file passed validation but won't open (most likely
            // disk full) — roll back to the pre-restore snapshot (M2).
            log::error!("Restored database failed to open: {e}");
            match rollback_to_snapshot(&config.db_path, &snapshot).await {
                Ok(pool) => {
                    *guard = pool;
                    Err(AppError::Internal(format!(
                        "Restore failed and was rolled back to your previous data: {e}"
                    )))
                }
                Err(re) => Err(AppError::Internal(format!(
                    "Restore failed ({e}) and automatic rollback also failed ({re}). \
                     Your previous data is in {}",
                    snapshot.to_string_lossy()
                ))),
            }
        }
    }
}

#[tauri::command]
pub async fn list_backups(app: AppHandle, _db: State<'_, Db>, auth_header: Option<String>) -> Result<Vec<BackupInfo>, AppError> {
    // NOTE: the parameter MUST stay `auth_header` — Tauri camelCases it to
    // `authHeader`, which is what the frontend sends (api-client.ts). Under
    // the old `_auth` name the token never arrived and this check would fail
    // every call (F1).
    require_admin(&app, auth_header).await?;

    let config = crate::config::CONFIG.clone();
    let mut backups = Vec::new();

    // Current exports live under %PROGRAMDATA%\MoonPipe\backups; older ones
    // sit directly in %PROGRAMDATA%\MoonPipe. Scan both, dedupe by path.
    let mut roots = vec![backups_dir(&config)];
    roots.push(config.app_data_dir.clone());

    for root in roots {
        let mut entries = match fs::read_dir(&root).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_backup =
                (name.starts_with("moonpipe_backup_") || name.starts_with("pre_restore_"))
                    && name.ends_with(".sqlite");
            if !is_backup {
                continue;
            }
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    backups.push(BackupInfo {
                        path: entry.path().to_string_lossy().to_string(),
                        name,
                        size: metadata.len(),
                        created: metadata
                            .created()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    });
                }
            }
        }
    }

    backups.sort_by_key(|b| std::cmp::Reverse(b.created));
    backups.dedup_by(|a, b| a.path == b.path);
    Ok(backups)
}

#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub created: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("moonpipe-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn remove_stale_wal_files_deletes_side_files_only() {
        let dir = temp_dir("wal-cleanup");
        let db = dir.join("moonpipe.db");
        let wal = dir.join("moonpipe.db-wal");
        let shm = dir.join("moonpipe.db-shm");

        std::fs::write(&db, b"db").unwrap();
        std::fs::write(&wal, b"wal").unwrap();
        std::fs::write(&shm, b"shm").unwrap();

        remove_stale_wal_files(&db).await.expect("cleanup must succeed");

        assert!(!wal.exists(), "stale -wal should be removed");
        assert!(!shm.exists(), "stale -shm should be removed");
        assert!(db.exists(), "the database file itself must survive");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn remove_stale_wal_files_is_harmless_when_nothing_exists() {
        let db =
            std::env::temp_dir().join(format!("moonpipe-wal-missing-{}.db", uuid::Uuid::new_v4()));
        // Must not panic on a database that was never opened in WAL mode.
        remove_stale_wal_files(&db)
            .await
            .expect("absent side files are not an error");
    }

    #[tokio::test]
    async fn remove_stale_wal_files_fails_when_side_file_cannot_be_deleted() {
        // M1: a side file that cannot go away (here: a directory named
        // `*-wal`) must surface as Err, never be swallowed — replaying a
        // leftover foreign WAL into the fresh database is the corruption path.
        let dir = temp_dir("wal-stuck");
        let db = dir.join("moonpipe.db");
        let wal = dir.join("moonpipe.db-wal");
        std::fs::write(&db, b"db").unwrap();
        std::fs::create_dir_all(&wal).unwrap(); // remove_file on a dir fails

        let err = remove_stale_wal_files(&db)
            .await
            .expect_err("undeletable -wal must be reported");
        assert!(err.contains("-wal"), "error must name the side file: {err}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_not_live_db_rejects_same_path_case_insensitively() {
        // H1: `MOONPIPE.DB` is the same file as `moonpipe.db` on Windows —
        // canonicalize collapses the case, the comparison must still reject.
        let dir = temp_dir("live-case");
        let live = dir.join("moonpipe.db");
        std::fs::write(&live, b"x").unwrap();
        let src = dir.join("MOONPIPE.DB");

        let err = ensure_not_live_db(&src, &live)
            .expect_err("restoring the live DB over itself must be rejected");
        assert!(
            matches!(err, AppError::Validation(_)),
            "must be a validation error, got: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_not_live_db_allows_same_name_in_another_folder() {
        // A backup copy on a USB stick may legitimately be named moonpipe.db.
        let dir_a = temp_dir("live-a");
        let dir_b = temp_dir("live-b");
        let live = dir_a.join("moonpipe.db");
        let src = dir_b.join("moonpipe.db");
        std::fs::write(&live, b"a").unwrap();
        std::fs::write(&src, b"b").unwrap();

        ensure_not_live_db(&src, &live).expect("same name in another folder is fine");

        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
    }

    #[tokio::test]
    async fn prepare_stage_folds_wal_sidecars_into_a_self_contained_file() {
        // H2 proof: commit a row whose frames live only in the -wal, stage
        // main + sidecars exactly the way import_database does, then show
        // prepare_stage produces a self-contained rollback-journal file that
        // still carries the row. Fails without the fix (header byte 18 == 2
        // and a leftover stage-wal, or a lost row).
        let dir = temp_dir("prepare-stage");
        let db = dir.join("source.db");
        let pool = crate::database::open_pool(&db).await.expect("migrated fixture");

        {
            let mut conn = pool.acquire().await.expect("acquire connection");
            sqlx::query("PRAGMA wal_autocheckpoint=0")
                .execute(&mut *conn)
                .await
                .expect("disable auto-checkpoint on this connection");
            sqlx::query(
                "INSERT INTO app_settings (key, value) VALUES ('prepare_stage_proof', '1')",
            )
            .execute(&mut *conn)
            .await
            .expect("commit the proof row");
        }

        let wal = dir.join("source.db-wal");
        assert!(
            wal.exists() && std::fs::metadata(&wal).unwrap().len() > 0,
            "the proof row must be sitting in an un-checkpointed -wal"
        );

        // Copy main + sidecars, mirroring import_database's staging step.
        let stage = dir.join("stage.sqlite");
        let stage_wal = PathBuf::from(format!("{}-wal", stage.to_string_lossy()));
        let stage_shm = PathBuf::from(format!("{}-shm", stage.to_string_lossy()));
        std::fs::copy(&db, &stage).unwrap();
        std::fs::copy(&wal, &stage_wal).unwrap();
        let shm = dir.join("source.db-shm");
        if shm.exists() {
            std::fs::copy(&shm, &stage_shm).unwrap();
        }
        pool.close().await;

        prepare_stage(&stage).await.expect("stage must be prepared");

        let bytes = std::fs::read(&stage).unwrap();
        assert_eq!(
            bytes[18], 1,
            "header must report a rollback journal (self-contained file)"
        );
        assert!(
            !stage_wal.exists() && !stage_shm.exists(),
            "no sidecar may survive prepare_stage"
        );

        let reopened = crate::database::open_pool(&stage)
            .await
            .expect("prepared stage must open");
        let proof: Option<String> =
            sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'prepare_stage_proof'")
                .fetch_one(&reopened)
                .await
                .expect("read the proof row");
        assert_eq!(
            proof.as_deref(),
            Some("1"),
            "the row that lived only in the -wal must survive"
        );
        reopened.close().await;

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn validate_stage_rejects_newer_migration_backup() {
        // M3: a backup carrying a migration beyond what this build embeds
        // would make sqlx refuse to open the live DB at next launch.
        let dir = temp_dir("validate-newer");
        let db = dir.join("future.sqlite");
        let pool = crate::database::open_pool(&db).await.expect("migrated fixture");

        let embedded_max: i64 = sqlx::migrate!("./src/database/migrations")
            .iter()
            .map(|m| m.version)
            .max()
            .unwrap_or(0);
        sqlx::query(
            "INSERT INTO _sqlx_migrations \
             (version, description, installed_on, success, checksum, execution_time) \
             VALUES (?1, 'from a newer app', '2026-10-08 00:00:00', 1, X'00', 0)",
        )
        .bind(embedded_max + 1)
        .execute(&pool)
        .await
        .expect("fake future migration");
        pool.close().await;

        prepare_stage(&db).await.expect("normalize fixture");
        let err = validate_stage(&db)
            .await
            .expect_err("backup from a newer app must be rejected");
        assert!(
            err.to_string().contains("newer"),
            "message must explain the version gap: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn validate_stage_accepts_backup_without_app_settings() {
        // M4: `app_settings` arrived with migration 016 — pre-016 backups
        // lack it and must still restore (import re-creates the table).
        let dir = temp_dir("validate-no-settings");
        let db = dir.join("old.sqlite");
        let pool = crate::database::open_pool(&db).await.expect("migrated fixture");
        sqlx::query("DROP TABLE app_settings")
            .execute(&pool)
            .await
            .expect("drop app_settings");
        pool.close().await;

        prepare_stage(&db).await.expect("normalize fixture");
        validate_stage(&db)
            .await
            .expect("backup without app_settings must pass validation");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn validate_stage_rejects_garbage_file() {
        let dir = temp_dir("validate-garbage");
        let file = dir.join("garbage.sqlite");
        std::fs::write(&file, b"this is definitely not a sqlite database").unwrap();

        validate_stage(&file)
            .await
            .expect_err("garbage bytes must be rejected");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn validate_stage_rejects_unrelated_sqlite_db() {
        // A valid SQLite file that simply is not a MoonPipe database.
        let dir = temp_dir("validate-unrelated");
        let file = dir.join("unrelated.sqlite");
        let url = format!(
            "sqlite:{}?mode=rwc",
            file.to_string_lossy().replace('\\', "/")
        );
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("create unrelated sqlite db");
        sqlx::query("CREATE TABLE foo (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .expect("create unrelated table");
        pool.close().await;

        let err = validate_stage(&file)
            .await
            .expect_err("unrelated sqlite db must be rejected");
        assert!(
            err.to_string().contains("MoonPipe"),
            "message must say it is not a MoonPipe database: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
