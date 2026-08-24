import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Download, CalendarRange } from "lucide-react";
import * as XLSX from "xlsx";
import { toast } from "sonner";

import { AdminShell } from "@/components/admin/AdminShell";
import { AdminOnly } from "@/components/admin/AdminOnly";
import { Button } from "@/components/ui/button";
import { currency } from "@/features/inventory/api";
import { useInvoices } from "@/features/invoices/api";
import { useInvoiceRealtime } from "@/lib/tauri-events";
import { formatDate } from "@/features/invoices/api";
import { saveWorkbook, fileStamp } from "@/lib/file-save";

export const Route = createFileRoute("/_authenticated/admin/reports")({
  component: () => (
    <AdminOnly title="Sales Reports">
      <ReportsPage />
    </AdminOnly>
  ),
  head: () => ({
    meta: [
      { title: "Sales Reports | City Tiles POS" },
      { name: "description", content: "Daily, weekly, monthly, six-month and yearly sales totals for City Tiles, with Excel export." },
      { property: "og:title", content: "Sales Reports | City Tiles POS" },
      { property: "og:description", content: "Sales performance overview and Excel exports for City Tiles." },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary" },
    ],
  }),
});

const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate());

function rangeStart(kind: "day" | "week" | "month" | "6month" | "year") {
  const now = new Date();
  switch (kind) {
    case "day":
      return startOfDay(now);
    case "week": {
      const s = startOfDay(now);
      s.setDate(s.getDate() - 6);
      return s;
    }
    case "month":
      return new Date(now.getFullYear(), now.getMonth(), 1);
    case "6month": {
      const s = new Date(now.getFullYear(), now.getMonth() - 5, 1);
      return s;
    }
    case "year":
      return new Date(now.getFullYear(), 0, 1);
  }
}

function within(invoices: { created_at: string; total: number; amount_paid: number }[], from: Date) {
  const t = from.getTime();
  return invoices.filter((i) => new Date(i.created_at).getTime() >= t);
}

const sum = (rows: { total: number; amount_paid: number }[], key: "total" | "amount_paid") =>
  rows.reduce((acc, i) => acc + Number(i[key]), 0);

async function exportRows(rows: { invoice_no: string; created_at: string; customer_name: string; payment_method: string; subtotal: number; discount: number; total: number; amount_paid: number; delivery_date: string | null }[], label: string) {
  const data = rows
    .slice()
    .sort((a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime())
    .map((i) => ({
      "Invoice No": i.invoice_no,
      Date: formatDate(i.created_at),
      Customer: i.customer_name,
      "Payment Method": i.payment_method,
      Subtotal: Number(i.subtotal),
      Discount: Number(i.discount),
      Total: Number(i.total),
      "Amount Paid": Number(i.amount_paid),
      Balance: Math.max(0, Number(i.total) - Number(i.amount_paid)),
      "Delivery Date": i.delivery_date ?? "",
    }));

  data.push({
    "Invoice No": "TOTAL",
    Date: "",
    Customer: `${rows.length} invoices`,
    "Payment Method": "",
    Subtotal: 0,
    Discount: 0,
    Total: sum(rows, "total"),
    "Amount Paid": sum(rows, "amount_paid"),
    Balance: sum(rows, "total") - sum(rows, "amount_paid"),
    "Delivery Date": "",
  } as never);

  const sheet = XLSX.utils.json_to_sheet(data);
  sheet["!cols"] = [{ wch: 14 }, { wch: 14 }, { wch: 24 }, { wch: 14 }, { wch: 12 }, { wch: 10 }, { wch: 12 }, { wch: 12 }, { wch: 12 }, { wch: 14 }];
  const book = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(book, sheet, label.slice(0, 30));
  const bytes = new Uint8Array(
    XLSX.write(book, { bookType: "xlsx", type: "array" }) as ArrayBuffer,
  );
  try {
    const path = await saveWorkbook(
      `city-tiles-${label.toLowerCase().replace(/\s+/g, "-")}-${fileStamp()}.xlsx`,
      bytes,
    );
    if (path) toast.success(`Exported to ${path}`);
  } catch (err) {
    toast.error(`Export failed: ${(err as Error).message}`);
  }
}

function ReportsPage() {
  useInvoiceRealtime();
  const { data: invoices = [] } = useInvoices();

  const periods = [
    { key: "day", label: "Today" },
    { key: "week", label: "Last 7 days" },
    { key: "month", label: "This month" },
    { key: "6month", label: "Last 6 months" },
    { key: "year", label: "This year" },
  ] as const;

  const sections = periods.map((p) => {
    const rows = within(invoices, rangeStart(p.key));
    return { ...p, rows, total: sum(rows, "total"), paid: sum(rows, "amount_paid") };
  });

  const now = new Date();
  const months = Array.from({ length: 12 }, (_, idx) => {
    const d = new Date(now.getFullYear(), now.getMonth() - (11 - idx), 1);
    const next = new Date(d.getFullYear(), d.getMonth() + 1, 1);
    const rows = invoices.filter((i) => {
      const t = new Date(i.created_at).getTime();
      return t >= d.getTime() && t < next.getTime();
    });
    return {
      label: d.toLocaleDateString("en-GB", { month: "short", year: "numeric" }),
      count: rows.length,
      total: sum(rows, "total"),
      paid: sum(rows, "amount_paid"),
      rows,
    };
  });

  const maxMonth = Math.max(1, ...months.map((m) => m.total));

  return (
    <AdminShell
      title="Sales reports"
      actions={
        <Button size="sm" variant="brass" onClick={() => exportRows(invoices, "All sales")}>
          <Download className="size-4" /> Export all
        </Button>
      }
    >
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
        {sections.map((s) => (
          <div key={s.key} className="rounded-lg border border-border bg-card p-5">
            <div className="flex items-center justify-between text-muted-foreground">
              <span className="text-xs uppercase tracking-wider">{s.label}</span>
              <CalendarRange className="size-4" />
            </div>
            <p className="mt-3 text-2xl font-semibold">{currency(s.total)}</p>
            <p className="mt-1 text-xs text-muted-foreground">
              {s.rows.length} invoices · {currency(s.paid)} received
            </p>
            <Button
              size="sm"
              variant="outline"
              className="mt-4 w-full"
              disabled={s.rows.length === 0}
              onClick={() => exportRows(s.rows, s.label)}
            >
              <Download className="size-4" /> Excel
            </Button>
          </div>
        ))}
      </div>

      <div className="mt-6 rounded-lg border border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 className="font-semibold">Monthly breakdown (last 12 months)</h2>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
                <th className="px-5 py-3">Month</th>
                <th className="px-2 py-3">Invoices</th>
                <th className="px-2 py-3">Sales</th>
                <th className="px-2 py-3">Received</th>
                <th className="px-2 py-3">Balance</th>
                <th className="px-2 py-3 w-40">Share</th>
                <th className="px-5 py-3 text-right">Export</th>
              </tr>
            </thead>
            <tbody>
              {months.map((m) => (
                <tr key={m.label} className="border-b border-border last:border-0">
                  <td className="px-5 py-3 font-medium">{m.label}</td>
                  <td className="px-2 py-3 text-muted-foreground">{m.count}</td>
                  <td className="px-2 py-3">{currency(m.total)}</td>
                  <td className="px-2 py-3 text-muted-foreground">{currency(m.paid)}</td>
                  <td className="px-2 py-3 text-muted-foreground">
                    {currency(Math.max(0, m.total - m.paid))}
                  </td>
                  <td className="px-2 py-3">
                    <div className="h-2 w-full rounded-full bg-muted">
                      <div
                        className="h-2 rounded-full bg-brass"
                        style={{ width: `${(m.total / maxMonth) * 100}%` }}
                      />
                    </div>
                  </td>
                  <td className="px-5 py-3 text-right">
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={m.count === 0}
                      onClick={() => exportRows(m.rows, m.label)}
                    >
                      <Download className="size-4" />
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </AdminShell>
  );
}