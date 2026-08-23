import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Search, Undo2 } from "lucide-react";
import { toast } from "sonner";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { currency } from "@/features/inventory/api";
import { useInvoices, type Invoice } from "@/features/invoices/api";
import { useReturns, useCreateReturn, type Return } from "@/features/returns/api";
import { formatDate } from "@/features/invoices/api";
import { type InvoiceItem } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/returns")({
  component: ReturnsPage,
});

type ReturnRow = {
  id: string;
  invoice_no: string;
  customer_name: string;
  total: number;
  reason: string | null;
  created_at: string;
};

function ReturnsPage() {
  const queryClient = useQueryClient();
  const [query, setQuery] = useState("");
  const [invoiceId, setInvoiceId] = useState("");
  const [qty, setQty] = useState<Record<string, string>>({});
  const [reason, setReason] = useState("");

  const { data: invoices = [] } = useInvoices();
  const { data: returns = [] } = useReturns();

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = q
      ? invoices.filter((i: Invoice) =>
          [i.invoice_no, i.customer_name].some((v) => String(v).toLowerCase().includes(q)),
        )
      : invoices;
    return list.slice(0, 8);
  }, [invoices, query]);

  const invoice = invoices.find((i: Invoice) => i.id === invoiceId) ?? null;

  const { data: detail } = useQuery({
    queryKey: ["return-invoice", invoiceId],
    enabled: !!invoiceId,
    queryFn: async () => {
      // In the new system, we'd call a Tauri command to get invoice items and returned quantities
      // For now, return empty data since we're in mock mode
      return { items: [] as InvoiceItem[], returnedBy: new Map<string, number>() };
    },
  });

  const items = detail?.items ?? [];
  const returnedBy = detail?.returnedBy ?? new Map<string, number>();
  const remainingOf = (item: InvoiceItem) =>
    Math.max(0, Number(item.quantity) - (returnedBy.get(item.product_id ?? item.product_name) ?? 0));

  const refundTotal = items.reduce(
    (acc, item) => acc + (Number(qty[item.id]) || 0) * Number(item.unit_price),
    0,
  );

  const createReturn = useCreateReturn();

  const submit = useMutation({
    mutationFn: async () => {
      const payload = items
        .map((item) => ({
          invoice_id: invoiceId,
          product_id: item.product_id,
          product_name: item.product_name,
          quantity: Number(qty[item.id]) || 0,
          unit: item.unit,
          unit_price: Number(item.unit_price),
          line_total: Number(item.unit_price) * (Number(qty[item.id]) || 0),
          reason: reason.trim(),
        }))
        .filter((row) => row.quantity > 0);
      if (payload.length === 0) throw new Error("Enter at least one returned quantity");
      for (const item of items) {
        const q = Number(qty[item.id]) || 0;
        if (q > remainingOf(item))
          throw new Error(`${item.product_name}: only ${remainingOf(item)} left to return`);
      }
      for (const p of payload) {
        await createReturn.mutateAsync(p);
      }
    },
    onSuccess: () => {
      toast.success("Return recorded — stock restored and sale adjusted");
      setQty({});
      setReason("");
      queryClient.invalidateQueries();
    },
    onError: (err: Error) => toast.error(err.message),
  });

  return (
    <AdminShell title="Returns">
      <div className="grid gap-5 lg:grid-cols-[1fr_1fr]">
        <div className="rounded-lg border border-border bg-card p-4">
          <h2 className="font-semibold">New return</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Pick the invoice, enter the returned quantity per item. Stock goes back to inventory and
            the sale total is reduced automatically.
          </p>

          <div className="relative mt-4">
            <Search className="absolute left-3 top-2.5 size-4 text-muted-foreground" />
            <Input
              className="pl-9"
              placeholder="Search invoice number or customer"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>

          <div className="mt-3 grid gap-2">
            {matches.map((i: Invoice) => (
              <button
                key={i.id}
                type="button"
                onClick={() => {
                  setInvoiceId(i.id);
                  setQty({});
                }}
                className={`rounded-md border p-3 text-left text-sm transition-colors ${
                  i.id === invoiceId ? "border-foreground" : "border-border hover:border-foreground"
                }`}
              >
                <span className="font-medium text-brass">{i.invoice_no}</span> · {i.customer_name}
                <span className="block text-xs text-muted-foreground">
                  {formatDate(i.created_at)} · {currency(i.total)}
                </span>
              </button>
            ))}
            {matches.length === 0 && (
              <p className="text-sm text-muted-foreground">No invoices match that search.</p>
            )}
          </div>

          {invoice && (
            <div className="mt-5 border-t border-border pt-4">
              <p className="text-sm font-medium">Items on {invoice.invoice_no}</p>
              <div className="mt-2 divide-y divide-border">
                {items.map((item) => {
                  const remaining = remainingOf(item);
                  return (
                    <div key={item.id} className="flex items-center gap-3 py-3">
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-sm">{item.product_name}</p>
                        <p className="text-xs text-muted-foreground">
                          Sold {item.quantity} {item.unit} · {remaining} returnable ·{" "}
                          {currency(item.unit_price)}
                        </p>
                      </div>
                      <Input
                        type="number"
                        min="0"
                        max={remaining}
                        disabled={remaining === 0}
                        className="w-24"
                        value={qty[item.id] ?? ""}
                        placeholder="0"
                        onChange={(e) => setQty((p) => ({ ...p, [item.id]: e.target.value }))}
                      />
                    </div>
                  );
                })}
                {items.length === 0 && (
                  <p className="py-3 text-sm text-muted-foreground">Loading items…</p>
                )}
              </div>

              <div className="mt-4 space-y-2">
                <Label>Reason (optional)</Label>
                <Input
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  placeholder="Damaged, wrong size, extra stock…"
                  maxLength={200}
                />
              </div>

              <div className="mt-4 flex items-center justify-between border-t border-border pt-4">
                <span className="text-sm text-muted-foreground">Return value</span>
                <span className="text-lg font-semibold">{currency(refundTotal)}</span>
              </div>
              <Button
                className="mt-3 w-full"
                variant="stone"
                disabled={submit.isPending || refundTotal <= 0}
                onClick={() => submit.mutate()}
              >
                <Undo2 className="size-4" /> Record return
              </Button>
            </div>
          )}
        </div>

        <div className="rounded-lg border border-border bg-card p-4">
          <h2 className="font-semibold">Recent returns</h2>
          <div className="mt-3 divide-y divide-border">
            {returns.map((r: Return) => (
              <div key={r.id} className="flex items-start justify-between gap-3 py-3 text-sm">
                <div className="min-w-0">
                  <p className="font-medium">
                    <span className="text-brass">{r.invoice_no}</span> · {r.customer_name}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {formatDate(r.created_at)}
                    {r.reason ? ` · ${r.reason}` : ""}
                  </p>
                </div>
                <span className="shrink-0 font-medium text-destructive">
                  -{currency(Number(r.total))}
                </span>
              </div>
            ))}
            {returns.length === 0 && (
              <p className="py-3 text-sm text-muted-foreground">No returns recorded yet.</p>
            )}
          </div>
        </div>
      </div>
    </AdminShell>
  );
}