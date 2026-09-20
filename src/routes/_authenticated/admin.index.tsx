import { createFileRoute, Link, Navigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { TrendingUp, Package, AlertTriangle, Wallet, DollarSign } from "lucide-react";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { currency } from "@/features/inventory/api";
import { useDashboard } from "@/features/reports/api";
import { useProducts, type Product } from "@/features/inventory/api";
import { useRoles } from "@/features/auth/api";
import { formatDate } from "@/features/invoices/api";
import { useInvoices, type Invoice } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/")({
  component: DashboardGate,
});

// Cashiers have no dashboard access — send them straight to the sales screen.
function DashboardGate() {
  const { data: roles = [], isLoading } = useRoles();
  const isAdmin = roles.includes("admin");
  if (isLoading) return null;
  if (!isAdmin) return <Navigate to="/admin/pos" replace />;
  return <DashboardPage />;
}

function sumBetween(invoices: { created_at: string; total: number }[], days: number) {
  const from = Date.now() - days * 24 * 60 * 60 * 1000;
  return invoices
    .filter((i) => new Date(i.created_at).getTime() >= from)
    .reduce((acc, i) => acc + Number(i.total), 0);
}

function DashboardPage() {
  const { data: dashboard } = useDashboard();
  const { data: invoices = [] } = useInvoices();
  const { data: products = [] } = useProducts();

  const today = invoices.filter(
    (i) => new Date(i.created_at).toDateString() === new Date().toDateString(),
  );
  const lowStock = products.filter((p) => p.stock_qty <= p.low_stock_threshold);
  const stockValue = products.reduce((acc, p) => acc + Number(p.price) * Math.max(0, Number(p.stock_qty)), 0);
  const outstanding = invoices.reduce(
    (acc, i) => acc + Math.max(0, Number(i.total) - Number(i.amount_paid)),
    0,
  );

  const bestSellers = products
    .filter((p) => p.stock_qty > 0)
    .slice()
    .sort((a, b) => Number(b.price) * b.stock_qty - Number(a.price) * a.stock_qty)
    .slice(0, 5);

  const cards = [
    { label: "Sales today", value: currency(sumBetween(invoices, 1)), sub: `${today.length} invoices`, icon: TrendingUp },
    { label: "Last 7 days", value: currency(sumBetween(invoices, 7)), sub: "rolling week", icon: TrendingUp },
    { label: "Stock value", value: currency(stockValue), sub: `${products.length} products`, icon: Package },
    { label: "Outstanding credit", value: currency(outstanding), sub: "unpaid balances", icon: Wallet },
    { label: "Profit today", value: currency(dashboard?.profit_today ?? 0), sub: "gross margin", icon: DollarSign },
    { label: "Profit 7 days", value: currency(dashboard?.profit_7d ?? 0), sub: "rolling week", icon: DollarSign },
    { label: "Profit 30 days", value: currency(dashboard?.profit_30d ?? 0), sub: "rolling month", icon: DollarSign },
  ];

  return (
    <AdminShell
      title="Dashboard"
      actions={
        <Button asChild size="sm" variant="brass">
          <Link to="/admin/pos">New sale</Link>
        </Button>
      }
    >
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {cards.map((c) => (
          <div key={c.label} className="rounded-lg border border-border bg-card p-5">
            <div className="flex items-center justify-between text-muted-foreground">
              <span className="text-xs uppercase tracking-wider">{c.label}</span>
              <c.icon className="size-4" />
            </div>
            <p className="mt-3 text-2xl font-semibold">{c.value}</p>
            <p className="mt-1 text-xs text-muted-foreground">{c.sub}</p>
          </div>
        ))}
      </div>

      <div className="mt-6 grid gap-6 lg:grid-cols-2">
        <div className="rounded-lg border border-border bg-card">
          <div className="flex items-center justify-between border-b border-border px-5 py-3">
            <h2 className="font-semibold">Recent invoices</h2>
            <Link to="/admin/invoices" className="text-xs text-muted-foreground hover:underline">
              View all
            </Link>
          </div>
          {invoices.length === 0 ? (
            <p className="p-5 text-sm text-muted-foreground">No sales recorded yet.</p>
          ) : (
            <table className="w-full text-sm">
              <tbody>
                {invoices.slice(0, 6).map((inv: Invoice) => (
                  <tr key={inv.id} className="border-b border-border last:border-0">
                    <td className="px-5 py-3 font-medium">{inv.invoice_no}</td>
                    <td className="px-2 py-3 text-muted-foreground">{inv.customer_name}</td>
                    <td className="px-2 py-3 text-muted-foreground">{formatDate(inv.created_at)}</td>
                    <td className="px-5 py-3 text-right">{currency(inv.total)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <div className="rounded-lg border border-border bg-card">
          <div className="flex items-center justify-between border-b border-border px-5 py-3">
            <h2 className="font-semibold">Low stock alerts</h2>
            <AlertTriangle className="size-4 text-destructive" />
          </div>
          {lowStock.length === 0 ? (
            <p className="p-5 text-sm text-muted-foreground">All products are above their threshold.</p>
          ) : (
            <ul className="divide-y divide-border text-sm">
              {lowStock.map((p: Product) => (
                <li key={p.id} className="flex items-center justify-between px-5 py-3">
                  <span>{p.name}</span>
                  <span className="text-destructive">
                    {p.stock_qty} {p.unit}
                  </span>
                </li>
              ))}
            </ul>
          )}
          <div className="border-t border-border px-5 py-3">
            <h3 className="text-xs uppercase tracking-wider text-muted-foreground">
              Highest stock value
            </h3>
            <ul className="mt-2 space-y-1 text-sm">
              {bestSellers.map((p: Product) => (
                <li key={p.id} className="flex justify-between">
                  <span className="text-muted-foreground">{p.name}</span>
                  <span>{currency(Number(p.price) * Math.max(0, p.stock_qty))}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </AdminShell>
  );
}