import { ShieldAlert } from "lucide-react";
import type { ReactNode } from "react";

import { useIsAdmin, useRoles } from "@/lib/pos";
import { AdminShell } from "@/components/admin/AdminShell";

export function AdminOnly({ title, children }: { title: string; children: ReactNode }) {
  const { isLoading } = useRoles();
  const isAdmin = useIsAdmin();

  if (isLoading) return null;
  if (isAdmin) return <>{children}</>;

  return (
    <AdminShell title={title}>
      <div className="mx-auto mt-10 max-w-md rounded-lg border border-border bg-card p-8 text-center">
        <ShieldAlert className="mx-auto size-8 text-brass" />
        <h2 className="mt-4 text-lg font-semibold">Admin access only</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          This section is restricted to the admin account. Use the Point of Sale screen to record sales.
        </p>
      </div>
    </AdminShell>
  );
}
