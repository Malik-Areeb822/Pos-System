// src-tauri/src/services/reconciliation.rs
//
// Recompute `customers.outstanding_balance` from the invoice ledger and
// `suppliers.outstanding_balance` from the purchase ledger.
//
// The carry-forward chain turns each customer's unpaid invoices into a
// linked list: an invoice whose balance was picked up by a newer one gets
// `carried_to_invoice_id` set, and only the final "leaf" of the chain still
// represents money the customer actually owes. The balance is therefore the
// SUM of the due amounts of all *non-carried* open invoices — a SUM, not a
// "newest one", because a customer can legitimately hold more than one open
// invoice when this feature first runs against pre-existing data.
//
// Supplier purchases behave identically with `carried_to_purchase_id`.

use crate::database::DbPool;
use crate::error::AppError;
use sqlx::Row;

/// One statement, two shapes: `{customer}` is the expression naming the
/// customer being reconciled and `{scope}` restricts which rows are touched.
/// Generating both statements from this template keeps the bulk (startup /
/// button) and single-customer (returns) paths from drifting apart.
const RECONCILE_TEMPLATE: &str = r#"UPDATE customers SET outstanding_balance = MAX(0, COALESCE((
    SELECT SUM(inv.total - inv.amount_paid) FROM invoices inv
    WHERE inv.customer_id = {customer}
      AND inv.carried_to_invoice_id IS NULL
      AND inv.total > inv.amount_paid), 0))
WHERE {scope}"#;

const CUSTOMERS_WITH_INVOICES: &str = "id IN (SELECT DISTINCT customer_id FROM invoices WHERE customer_id IS NOT NULL)";

fn sql_reconcile_all() -> String {
    RECONCILE_TEMPLATE
        .replace("{customer}", "customers.id")
        .replace("{scope}", CUSTOMERS_WITH_INVOICES)
}

fn sql_reconcile_one() -> String {
    RECONCILE_TEMPLATE
        .replace("{customer}", "?")
        .replace("{scope}", "id = ?")
}

/// Supplier mirror of `RECONCILE_TEMPLATE`. Same shape, same reasons:
/// `{supplier}` names the supplier, `{scope}` restricts which rows are
/// touched, and the SUM excludes purchases whose balance was carried away
/// so absorbed rows are never double counted.
const SUPPLIER_RECONCILE_TEMPLATE: &str = r#"UPDATE suppliers SET outstanding_balance = MAX(0, COALESCE((
    SELECT SUM(p.total - p.amount_paid) FROM supplier_purchases p
    WHERE p.supplier_id = {supplier}
      AND p.carried_to_purchase_id IS NULL
      AND p.total > p.amount_paid), 0))
WHERE {scope}"#;

const SUPPLIERS_WITH_PURCHASES: &str =
    "id IN (SELECT DISTINCT supplier_id FROM supplier_purchases WHERE supplier_id IS NOT NULL)";

fn sql_reconcile_suppliers_all() -> String {
    SUPPLIER_RECONCILE_TEMPLATE
        .replace("{supplier}", "suppliers.id")
        .replace("{scope}", SUPPLIERS_WITH_PURCHASES)
}

fn sql_reconcile_supplier_one() -> String {
    SUPPLIER_RECONCILE_TEMPLATE
        .replace("{supplier}", "?")
        .replace("{scope}", "id = ?")
}

/// Recompute every customer's outstanding_balance from the actual invoice ledger.
///
/// Walk-in customers (customer_id = NULL) and customers with no invoices are
/// untouched. Returns the number of customer rows the UPDATE matched.
pub async fn reconcile_balances(pool: &DbPool) -> Result<i64, AppError> {
    let result = sqlx::query(&sql_reconcile_all())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() as i64)
}

/// Recompute a single customer's balance. Takes a connection so callers can
/// run it inside their own transaction alongside the writes it must reflect.
pub async fn reconcile_customer(
    conn: &mut sqlx::SqliteConnection,
    customer_id: &str,
) -> Result<(), AppError> {
    // The template binds twice: once in the correlated subquery, once in the
    // WHERE. Binding only one would leave the second as NULL and silently
    // match no rows — the balance would simply never change.
    sqlx::query(&sql_reconcile_one())
        .bind(customer_id)
        .bind(customer_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Recompute every supplier's outstanding_balance from the purchase ledger.
///
/// Suppliers with no purchases are untouched. Returns the number of supplier
/// rows the UPDATE matched.
pub async fn reconcile_suppliers(pool: &DbPool) -> Result<i64, AppError> {
    let result = sqlx::query(&sql_reconcile_suppliers_all())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() as i64)
}

/// Recompute a single supplier's balance. Takes a connection so callers can
/// run it inside their own transaction alongside the writes it must reflect.
pub async fn reconcile_supplier(
    conn: &mut sqlx::SqliteConnection,
    supplier_id: &str,
) -> Result<(), AppError> {
    // Bound twice, exactly like the customer path: once in the correlated
    // subquery, once in the WHERE.
    sqlx::query(&sql_reconcile_supplier_one())
        .bind(supplier_id)
        .bind(supplier_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Invariant A: at most one open, non-carried invoice per customer. Nothing in
/// the app can violate it after the create()-side fix, but restored, imported
/// or hand-edited databases still can — and a violation means a balance that
/// does not match the ledger. Returns one row per offending customer.
async fn broken_invariant_rows(
    pool: &DbPool,
) -> Result<Vec<(String, String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT c.id, c.name, COUNT(*) AS open_count
           FROM customers c
           JOIN invoices i ON i.customer_id = c.id
           WHERE i.total > i.amount_paid AND i.carried_to_invoice_id IS NULL
           GROUP BY c.id, c.name
           HAVING COUNT(*) > 1"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("id"),
                r.get::<String, _>("name"),
                r.get::<i64, _>("open_count"),
            )
        })
        .collect())
}

/// Supplier mirror of invariant A: at most one open, non-carried purchase per
/// supplier. A violation means the reported balance may not match the ledger.
/// Returns one row per offending supplier.
async fn broken_supplier_invariant_rows(
    pool: &DbPool,
) -> Result<Vec<(String, String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT s.id, s.name, COUNT(*) AS open_count
           FROM suppliers s
           JOIN supplier_purchases p ON p.supplier_id = s.id
           WHERE p.total > p.amount_paid AND p.carried_to_purchase_id IS NULL
           GROUP BY s.id, s.name
           HAVING COUNT(*) > 1"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("id"),
                r.get::<String, _>("name"),
                r.get::<i64, _>("open_count"),
            )
        })
        .collect())
}

/// Log loudly when invariant A is broken; this is otherwise invisible.
async fn warn_on_broken_invariant(pool: &DbPool) {
    match broken_invariant_rows(pool).await {
        Ok(rows) if !rows.is_empty() => {
            let summary: Vec<String> = rows
                .iter()
                .map(|(_, name, open_count)| format!("{} ({} open invoices)", name, open_count))
                .collect();
            log::warn!(
                "Carry-forward invariant broken — {} customer(s) hold multiple open invoices; \
                 their reported balance may not match the ledger: {}",
                rows.len(),
                summary.join("; ")
            );
        }
        Ok(_) => {}
        Err(e) => log::warn!("Could not check carry-forward invariant: {}", e),
    }

    match broken_supplier_invariant_rows(pool).await {
        Ok(rows) if !rows.is_empty() => {
            let summary: Vec<String> = rows
                .iter()
                .map(|(_, name, open_count)| format!("{} ({} open purchases)", name, open_count))
                .collect();
            log::warn!(
                "Supplier carry-forward invariant broken — {} supplier(s) hold multiple open \
                 purchases; their reported balance may not match the ledger: {}",
                rows.len(),
                summary.join("; ")
            );
        }
        Ok(_) => {}
        Err(e) => log::warn!("Could not check supplier carry-forward invariant: {}", e),
    }
}

/// Fire-and-forget wrapper: reconcile, warn if the invariant is still broken,
/// then notify the UI only if rows actually changed. Never panics the app.
///
/// Called from app startup (`lib.rs`) and after a backup restore (`backup.rs`).
pub async fn run_and_notify(app: tauri::AppHandle, pool: DbPool) {
    match reconcile_balances(&pool).await {
        Ok(n) => {
            if n > 0 {
                log::info!("Reconciled {} customer balances", n);
                crate::events::emit_customers_changed(&app).await;
            }
        }
        Err(e) => log::error!("Balance reconciliation failed: {}", e),
    }

    match reconcile_suppliers(&pool).await {
        Ok(n) => {
            if n > 0 {
                log::info!("Reconciled {} supplier balances", n);
                crate::events::emit_suppliers_changed(&app).await;
            }
        }
        Err(e) => log::error!("Supplier balance reconciliation failed: {}", e),
    }

    warn_on_broken_invariant(&pool).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[tokio::test]
    async fn reconcile_sets_balance_from_a_single_open_invoice() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Ayesha").await;
        credit_invoice(&pool, &c, vec![item("Wall-Hung WC", 1, 1000)]).await;

        // Clobber the cached balance, then recompute it from the ledger.
        sqlx::query("UPDATE customers SET outstanding_balance = 0 WHERE id = ?")
            .bind(&c.id)
            .execute(&pool)
            .await
            .unwrap();

        reconcile_balances(&pool).await.unwrap();
        assert_eq!(balance(&pool, &c.id).await, 1000);
    }

    #[tokio::test]
    async fn reconcile_sums_every_open_leaf_not_just_the_newest() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Bilal").await;

        // Legacy shape: two open invoices neither of which was ever carried --
        // exactly what a database migrated from the pre-fix build contains.
        raw_open_invoice(&pool, Some(&c.id), "Bilal", 1120, 0, 0, None).await;
        raw_open_invoice(&pool, Some(&c.id), "Bilal", 880, 0, 0, None).await;

        reconcile_balances(&pool).await.unwrap();
        assert_eq!(balance(&pool, &c.id).await, 2000);
        assert_eq!(
            balance(&pool, &c.id).await,
            expected_leaf_balance(&pool, &c.id).await
        );
    }

    #[tokio::test]
    async fn reconcile_excludes_invoices_whose_balance_was_carried_away() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Hina").await;

        let absorbed = raw_open_invoice(&pool, Some(&c.id), "Hina", 1000, 0, 0, None).await;
        let successor = raw_open_invoice(&pool, Some(&c.id), "Hina", 1500, 1000, 0, None).await;
        sqlx::query("UPDATE invoices SET carried_to_invoice_id = ? WHERE id = ?")
            .bind(&successor)
            .bind(&absorbed)
            .execute(&pool)
            .await
            .unwrap();

        // 1000 + 1500 would double count the same money; only the leaf counts.
        reconcile_balances(&pool).await.unwrap();
        assert_eq!(balance(&pool, &c.id).await, 1500);
    }

    #[tokio::test]
    async fn reconcile_is_stable_across_runs() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Sana").await;
        credit_invoice(&pool, &c, vec![item("Mixer", 2, 500)]).await;

        reconcile_balances(&pool).await.unwrap();
        let first = balance(&pool, &c.id).await;

        // A second run must not drift. (Guards against reasoning from
        // rows_affected: it is legitimately 0 when the value is already right.)
        reconcile_balances(&pool).await.unwrap();
        assert_eq!(balance(&pool, &c.id).await, first);
        assert_eq!(first, 1000);
    }

    #[tokio::test]
    async fn reconcile_leaves_walkins_and_invoiceless_customers_alone() {
        let pool = test_pool().await;

        let empty = make_customer(&pool, "Unrelated").await;
        sqlx::query("UPDATE customers SET outstanding_balance = 777 WHERE id = ?")
            .bind(&empty.id)
            .execute(&pool)
            .await
            .unwrap();

        // Walk-in sale: no customer row to attribute it to.
        raw_open_invoice(&pool, None, "Walk-in", 500, 0, 0, None).await;

        reconcile_balances(&pool).await.unwrap();
        assert_eq!(balance(&pool, &empty.id).await, 777);
    }

    #[tokio::test]
    async fn reconcile_customer_only_touches_its_own_row() {
        let pool = test_pool().await;
        let target = make_customer(&pool, "Target").await;
        let other = make_customer(&pool, "Other").await;

        credit_invoice(&pool, &target, vec![item("Tile", 1, 900)]).await;
        sqlx::query("UPDATE customers SET outstanding_balance = 12345 WHERE id = ?")
            .bind(&other.id)
            .execute(&pool)
            .await
            .unwrap();

        let mut conn = pool.acquire().await.unwrap();
        reconcile_customer(&mut conn, &target.id).await.unwrap();
        drop(conn);

        assert_eq!(balance(&pool, &target.id).await, 900);
        assert_eq!(balance(&pool, &other.id).await, 12345);
    }

    #[tokio::test]
    async fn invariant_violation_is_detected() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Duplicator").await;

        raw_open_invoice(&pool, Some(&c.id), "Duplicator", 1000, 0, 0, None).await;
        assert!(broken_invariant_rows(&pool).await.unwrap().is_empty());

        raw_open_invoice(&pool, Some(&c.id), "Duplicator", 500, 0, 0, None).await;
        let rows = broken_invariant_rows(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, c.id);
        assert_eq!(rows[0].2, 2);

        // Carrying one away restores the invariant.
        let successor =
            raw_open_invoice(&pool, Some(&c.id), "Duplicator", 1500, 1500, 0, None).await;
        sqlx::query(
            "UPDATE invoices SET carried_to_invoice_id = ? WHERE customer_id = ? AND id != ?",
        )
        .bind(&successor)
        .bind(&c.id)
        .bind(&successor)
        .execute(&pool)
        .await
        .unwrap();
        assert!(broken_invariant_rows(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn supplier_reconcile_sets_balance_from_ledger() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Reconciled Trading").await;
        credit_purchase(&pool, &s, vec![purchase_item("Basin", 1, 1200)]).await;

        // Clobber the cached balance, then recompute it from the ledger.
        sqlx::query("UPDATE suppliers SET outstanding_balance = 0 WHERE id = ?")
            .bind(&s.id)
            .execute(&pool)
            .await
            .unwrap();

        reconcile_suppliers(&pool).await.unwrap();
        assert_eq!(supplier_balance(&pool, &s.id).await, 1200);
    }

    #[tokio::test]
    async fn supplier_reconcile_excludes_carried_purchases() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Double Count").await;

        let absorbed = raw_open_purchase(&pool, &s.id, &s.name, 1000, 0, 0, None).await;
        let successor = raw_open_purchase(&pool, &s.id, &s.name, 1500, 1000, 0, None).await;
        sqlx::query("UPDATE supplier_purchases SET carried_to_purchase_id = ? WHERE id = ?")
            .bind(&successor)
            .bind(&absorbed)
            .execute(&pool)
            .await
            .unwrap();

        // 1000 + 1500 would double count the same money; only the leaf counts.
        reconcile_suppliers(&pool).await.unwrap();
        assert_eq!(supplier_balance(&pool, &s.id).await, 1500);
    }

    #[tokio::test]
    async fn supplier_reconcile_is_stable_across_runs() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Steady Vendor").await;
        credit_purchase(&pool, &s, vec![purchase_item("Pipe", 4, 250)]).await;

        reconcile_suppliers(&pool).await.unwrap();
        let first = supplier_balance(&pool, &s.id).await;

        reconcile_suppliers(&pool).await.unwrap();
        assert_eq!(supplier_balance(&pool, &s.id).await, first);
        assert_eq!(first, 1000);
    }

    #[tokio::test]
    async fn supplier_invariant_violation_is_detected() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Duplicator Supplies").await;

        raw_open_purchase(&pool, &s.id, &s.name, 1000, 0, 0, None).await;
        assert!(broken_supplier_invariant_rows(&pool).await.unwrap().is_empty());

        raw_open_purchase(&pool, &s.id, &s.name, 500, 0, 0, None).await;
        let rows = broken_supplier_invariant_rows(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, s.id);
        assert_eq!(rows[0].2, 2);

        // Carrying one away restores the invariant.
        let successor =
            raw_open_purchase(&pool, &s.id, &s.name, 1500, 1500, 0, None).await;
        sqlx::query(
            "UPDATE supplier_purchases SET carried_to_purchase_id = ? WHERE supplier_id = ? AND id != ?",
        )
        .bind(&successor)
        .bind(&s.id)
        .bind(&successor)
        .execute(&pool)
        .await
        .unwrap();
        assert!(broken_supplier_invariant_rows(&pool).await.unwrap().is_empty());
    }
}