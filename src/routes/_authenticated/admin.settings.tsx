import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";

import { AdminShell } from "@/components/admin/AdminShell";
import { AdminOnly } from "@/components/admin/AdminOnly";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { api, type BackupInfo } from "@/lib/api-client";
import { formatDate } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/settings")({
  component: () => (
    <AdminOnly title="Settings">
      <SettingsPage />
    </AdminOnly>
  ),
  head: () => ({
    meta: [
      { title: "Settings | Moon Pipe POS" },
      { name: "description", content: "Backup and restore the Moon Pipe POS database." },
    ],
  }),
});

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function SettingsPage() {
  const queryClient = useQueryClient();
  const [pendingRestore, setPendingRestore] = useState<BackupInfo | null>(null);

  const backups = useQuery({
    queryKey: ["backups"],
    queryFn: () => api.backups.list(),
  });

  const exportBackup = useMutation({
    mutationFn: () => api.backups.export(),
    onSuccess: (path) => toast.success(`Backup saved to ${path}`),
    onError: (err: Error) => toast.error(err.message),
  });

  async function pickBackupFile() {
    const selected = await openFileDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "Database backup", extensions: ["sqlite", "db"] }],
    });
    if (typeof selected !== "string") return;
    const name = selected.split(/[\\/]/).pop() ?? selected;
    setPendingRestore({ path: selected, name, size: 0, created: 0 });
  }

  const importBackup = useMutation({
    mutationFn: (path: string) => api.backups.import(path),
    onSuccess: () => {
      toast.success("Backup restored successfully — reloading app");
      queryClient.cancelQueries();
      setTimeout(() => window.location.reload(), 800);
    },
    onError: (err: Error) => toast.error(err.message),
    onSettled: () => setPendingRestore(null),
  });

  return (
    <AdminShell title="Settings">
      <div className="grid gap-6 lg:grid-cols-2">
        <section className="rounded-lg border border-border bg-card p-5">
          <h2 className="font-semibold">Export backup</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Creates a timestamped snapshot of the entire database under{" "}
            <code className="text-xs">%PROGRAMDATA%\MoonPipe\backups</code>. Safe to run while the
            shop is open.
          </p>
          <Button
            className="mt-4"
            size="sm"
            variant="brass"
            disabled={exportBackup.isPending}
            onClick={() => exportBackup.mutate()}
          >
            {exportBackup.isPending ? "Exporting…" : "Create backup"}
          </Button>
        </section>

        <section className="rounded-lg border border-destructive/40 bg-card p-5">
          <h2 className="font-semibold">Import / restore</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Replaces the live database with a backup file. A pre-restore snapshot of current data is
            saved automatically. All users should stop making sales during the restore.
          </p>
          <Button
            className="mt-4"
            size="sm"
            variant="outline"
            onClick={() => void pickBackupFile()}
          >
            Choose backup file…
          </Button>
        </section>
      </div>

      <section className="mt-6 overflow-x-auto rounded-lg border border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 className="font-semibold">Available backups</h2>
          <button
            className="text-xs text-muted-foreground hover:text-foreground hover:underline"
            onClick={() => void backups.refetch()}
          >
            Refresh
          </button>
        </div>
        {backups.isLoading ? (
          <p className="px-5 py-6 text-sm text-muted-foreground">Loading…</p>
        ) : (backups.data ?? []).length === 0 ? (
          <p className="px-5 py-6 text-sm text-muted-foreground">
            No backups yet — create one above.
          </p>
        ) : (
          <table className="w-full min-w-[560px] text-sm">
            <thead className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="px-5 py-3">File</th>
                <th className="px-3 py-3">Created</th>
                <th className="px-3 py-3 text-right">Size</th>
                <th className="px-5 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {(backups.data ?? []).map((b) => (
                <tr key={b.path} className="border-b border-border last:border-0">
                  <td className="px-5 py-3 font-medium">{b.name}</td>
                  <td className="px-3 py-3 text-muted-foreground">
                    {b.created > 0 ? formatDate(new Date(b.created * 1000).toISOString()) : "—"}
                  </td>
                  <td className="px-3 py-3 text-right text-muted-foreground">
                    {formatBytes(b.size)}
                  </td>
                  <td className="px-5 py-3">
                    <div className="flex justify-end gap-2">
                      <Button size="sm" variant="outline" onClick={() => setPendingRestore(b)}>
                        Restore
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <AlertDialog
        open={!!pendingRestore}
        onOpenChange={(openState) => !openState && setPendingRestore(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Replace current data?</AlertDialogTitle>
            <AlertDialogDescription>
              Restoring{" "}
              <span className="font-medium text-foreground">{pendingRestore?.name}</span> will{" "}
              <span className="font-semibold text-destructive">
                permanently replace every invoice, product, customer and user account
              </span>{" "}
              in the live database with the contents of this backup. This cannot be undone.
              {"\n\n"}Are you sure you want to continue?
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={importBackup.isPending || !pendingRestore}
              onClick={() => pendingRestore && importBackup.mutate(pendingRestore.path)}
            >
              {importBackup.isPending ? "Restoring…" : "Yes, restore backup"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </AdminShell>
  );
}
