import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";

import { AdminShell } from "@/components/admin/AdminShell";
import { AdminOnly } from "@/components/admin/AdminOnly";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCashiers, useApproveCashier, useRejectCashier, useSuspendCashier, useResetCashierPassword, type StaffProfile } from "@/features/cashiers/api";
import { formatDate } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/cashiers")({
  component: () => (
    <AdminOnly title="Cashier Management">
      <CashiersPage />
    </AdminOnly>
  ),
  head: () => ({
    meta: [
      { title: "Cashier Management | Moon Pipe POS" },
      {
        name: "description",
        content: "Approve, reject, suspend or reactivate cashier accounts for the Moon Pipe POS.",
      },
      { property: "og:title", content: "Cashier Management | Moon Pipe POS" },
      { property: "og:description", content: "Admin approval queue for Moon Pipe cashier accounts." },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary" },
    ],
  }),
});

const TABS: { key: StaffProfile["status"]; label: string }[] = [
  { key: "pending", label: "Pending requests" },
  { key: "approved", label: "Active cashiers" },
  { key: "suspended", label: "Suspended" },
  { key: "rejected", label: "Rejected" },
];

function CashiersPage() {
  const queryClient = useQueryClient();
  const { data: user } = useQuery({ queryKey: ["auth-me"], queryFn: () => null });
  const [tab, setTab] = useState<StaffProfile["status"]>("pending");

  const { data: staff = [], isLoading } = useCashiers();

  const approve = useApproveCashier();
  const reject = useRejectCashier();
  const suspend = useSuspendCashier();
  const resetPassword = useResetCashierPassword();

  const [resetTarget, setResetTarget] = useState<StaffProfile | null>(null);
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");

  const rows = staff.filter((s: StaffProfile) => s.status === tab && s.id !== user?.id);
  const pendingCount = staff.filter((s) => s.status === "pending" && s.id !== user?.id).length;

  function act(id: string, status: StaffProfile["status"]) {
    switch (status) {
      case "approved":
        approve.mutate(id);
        break;
      case "rejected":
        reject.mutate(id);
        break;
      case "suspended":
        suspend.mutate(id);
        break;
    }
  }

  function openResetDialog(cashier: StaffProfile) {
    setResetTarget(cashier);
    setNewPassword("");
    setConfirmPassword("");
  }

  function submitReset() {
    if (!resetTarget) return;
    if (newPassword.length < 6) {
      toast.error("New password must be at least 6 characters");
      return;
    }
    if (newPassword !== confirmPassword) {
      toast.error("Passwords do not match");
      return;
    }
    const target = resetTarget;
    resetPassword.mutate(
      { id: target.id, newPassword },
      {
        onSuccess: () => {
          toast.success(`Password reset for ${target.full_name || "cashier"}`);
          setResetTarget(null);
        },
        onError: (err: Error) => toast.error(err.message),
      },
    );
  }

  return (
    <AdminShell title="Cashier Management">
      <div className="flex flex-wrap gap-2">
        {TABS.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`rounded-md border px-3 py-2 text-sm transition-colors ${
              tab === t.key
                ? "border-foreground bg-foreground text-background"
                : "border-border bg-card text-muted-foreground hover:text-foreground"
            }`}
          >
            {t.label}
            {t.key === "pending" && pendingCount > 0 ? ` (${pendingCount})` : ""}
          </button>
        ))}
      </div>

      <div className="mt-5 overflow-x-auto rounded-lg border border-border bg-card">
        <table className="w-full min-w-[720px] text-sm">
          <thead className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
            <tr>
              <th className="px-5 py-3">Name</th>
              <th className="px-3 py-3">Email</th>
              <th className="px-3 py-3">Phone</th>
              <th className="px-3 py-3">Employee ID</th>
              <th className="px-3 py-3">Registered</th>
              <th className="px-3 py-3">Status</th>
              <th className="px-5 py-3 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {isLoading && (
              <tr>
                <td colSpan={7} className="px-5 py-6 text-muted-foreground">
                  Loading…
                </td>
              </tr>
            )}
            {!isLoading && rows.length === 0 && (
              <tr>
                <td colSpan={7} className="px-5 py-6 text-muted-foreground">
                  No accounts in this list.
                </td>
              </tr>
            )}
            {rows.map((s: StaffProfile) => (
              <tr key={s.id} className="border-b border-border last:border-0">
                <td className="px-5 py-3 font-medium">{s.full_name || "—"}</td>
                <td className="px-3 py-3 text-muted-foreground">{s.email ?? "—"}</td>
                <td className="px-3 py-3 text-muted-foreground">{s.phone ?? "—"}</td>
                <td className="px-3 py-3 text-muted-foreground">{s.employee_id ?? "—"}</td>
                <td className="px-3 py-3 text-muted-foreground">{formatDate(s.created_at)}</td>
                <td className="px-3 py-3 uppercase tracking-wider text-xs">{s.status}</td>
                <td className="px-5 py-3">
                  <div className="flex justify-end gap-2">
                    {s.status === "pending" && (
                      <>
                        <Button size="sm" variant="brass" onClick={() => act(s.id, "approved")}>
                          Approve
                        </Button>
                        <Button size="sm" variant="outline" onClick={() => act(s.id, "rejected")}>
                          Reject
                        </Button>
                      </>
                    )}
                    {s.status === "approved" && (
                      <>
                        <Button size="sm" variant="outline" onClick={() => act(s.id, "suspended")}>
                          Suspend
                        </Button>
                        <Button size="sm" variant="outline" onClick={() => openResetDialog(s)}>
                          Reset password
                        </Button>
                      </>
                    )}
                    {(s.status === "suspended" || s.status === "rejected") && (
                      <Button size="sm" variant="brass" onClick={() => act(s.id, "approved")}>
                        Reactivate
                      </Button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <Dialog open={!!resetTarget} onOpenChange={(open) => !open && setResetTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Reset password</DialogTitle>
            <DialogDescription>
              Set a new password for {resetTarget?.full_name || "this cashier"}. They will use it on
              their next sign-in.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1">
              <Label htmlFor="reset-new-password">New password</Label>
              <Input
                id="reset-new-password"
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="reset-confirm-password">Confirm password</Label>
              <Input
                id="reset-confirm-password"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter className="gap-2">
            <Button size="sm" variant="outline" onClick={() => setResetTarget(null)}>
              Cancel
            </Button>
            <Button
              size="sm"
              variant="brass"
              disabled={resetPassword.isPending || newPassword.length === 0}
              onClick={submitReset}
            >
              {resetPassword.isPending ? "Saving…" : "Reset password"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </AdminShell>
  );
}