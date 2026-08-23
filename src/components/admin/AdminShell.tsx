import { Link, useNavigate } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import {
  LayoutDashboard,
  ScanBarcode,
  Boxes,
  Users,
  ReceiptText,
  Undo2,
  LogOut,
  Store,
  BarChart3,
  UserCog,
  Settings,
} from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { useAuthStore } from "@/lib/auth-store";
import { useRoles, useSession } from "@/lib/pos";

const NAV = [
  { to: "/admin", label: "Dashboard", icon: LayoutDashboard, exact: true, adminOnly: true },
  { to: "/admin/pos", label: "Point of Sale", icon: ScanBarcode },
  { to: "/admin/inventory", label: "Inventory", icon: Boxes },
  { to: "/admin/customers", label: "Customers", icon: Users },
  { to: "/admin/invoices", label: "Invoices", icon: ReceiptText },
  { to: "/admin/returns", label: "Returns", icon: Undo2 },
  { to: "/admin/reports", label: "Reports", icon: BarChart3, adminOnly: true },
  { to: "/admin/cashiers", label: "Cashiers", icon: UserCog, adminOnly: true },
  { to: "/admin/settings", label: "Settings", icon: Settings, adminOnly: true },
];

export function AdminShell({
  title,
  actions,
  children,
}: {
  title: string;
  actions?: ReactNode;
  children: ReactNode;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data: user } = useSession();
  const { data: roles } = useRoles();
  const isAdmin = (roles ?? []).includes("admin");
  const nav = NAV.filter((item) => !item.adminOnly || isAdmin);

  async function signOut() {
    await queryClient.cancelQueries();
    queryClient.clear();
    useAuthStore.getState().clearAuth();
    navigate({ to: "/", replace: true });
  }

  return (
    <div className="min-h-screen bg-muted/40 lg:flex">
      <aside className="border-b border-border bg-card print:hidden lg:min-h-screen lg:w-60 lg:border-b-0 lg:border-r">
        <div className="flex items-center gap-2 px-5 py-5">
          <Store className="size-5 text-brass" />
          <span className="font-display text-lg leading-none">City Tiles POS</span>
        </div>
        <nav className="flex gap-1 overflow-x-auto px-3 pb-3 lg:flex-col lg:overflow-visible">
          {nav.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              activeOptions={{ exact: item.exact ?? false }}
              activeProps={{ className: "bg-foreground text-background" }}
              className="flex shrink-0 items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              <item.icon className="size-4" />
              {item.label}
            </Link>
          ))}
        </nav>
        <div className="hidden px-5 py-4 text-xs text-muted-foreground lg:block">
          <p className="truncate">{user?.email}</p>
          <p className="mt-1 uppercase tracking-wider">{roles?.join(" · ") || "staff"}</p>
        </div>
      </aside>

      <div className="flex-1">
        <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border bg-card px-5 py-4 print:hidden">
          <h1 className="text-xl font-semibold">{title}</h1>
          <div className="flex items-center gap-2">
            {actions}
            <Button variant="outline" size="sm" onClick={signOut}>
              <LogOut className="size-4" /> Sign out
            </Button>
          </div>
        </header>
        <main className="p-5 print:p-0">{children}</main>
      </div>
    </div>
  );
}