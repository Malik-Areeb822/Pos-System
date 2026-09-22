import { createFileRoute, Link } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";
import { ArrowRight } from "lucide-react";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { NumberInput } from "@/components/ui/number-input";
import { Label } from "@/components/ui/label";
import { BUSINESS } from "@/lib/business";

import { currency } from "@/features/inventory/api";
import { useInvoice, useMarkInvoicePaid, usePrintInvoicePdf, usePrintReceipt, type Invoice, type InvoiceItem } from "@/features/invoices/api";
import { formatDate } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/invoices/$invoiceId")({
  component: InvoiceDetailPage,
});

function InvoiceDetailPage() {
  const { invoiceId } = Route.useParams();
  const queryClient = useQueryClient();
  const { data, isLoading } = useInvoice(invoiceId);

  const invoice = data?.invoice;
  const items = data?.items ?? [];
  const balance = invoice ? Math.max(0, Number(invoice.total) - Number(invoice.amount_paid)) : 0;

  const markPaid = useMarkInvoicePaid();
  const printPdf = usePrintInvoicePdf();
  const printReceipt = usePrintReceipt();
  const [paymentAmount, setPaymentAmount] = useState("");

  const recordPayment = useMutation({
    mutationFn: () => {
      if (!invoice) throw new Error("Invoice not loaded");
      const entered = Number(paymentAmount);
      const amount = Number.isFinite(entered) && entered > 0 ? entered : balance;
      if (amount > balance) throw new Error("Payment cannot exceed the balance due");
      return markPaid.mutateAsync({
        id: invoice.id,
        amount,
        payment_method: invoice.payment_method,
      });
    },
    onError: (err: Error) => toast.error(err.message),
    onSuccess: (_data, _vars) => {
      const applied = Number(paymentAmount) > 0 ? Number(paymentAmount) : balance;
      const remaining = Math.max(0, balance - applied);
      toast.success(
        remaining > 0
          ? `Payment recorded — ${currency(remaining)} still due`
          : "Invoice fully paid"
      );
      setPaymentAmount("");
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["invoice", invoiceId] });
      queryClient.invalidateQueries({ queryKey: ["invoices"] });
      queryClient.invalidateQueries({ queryKey: ["customers"] });
    },
  });

  return (
    <AdminShell
      title="Invoice"
          actions={
            <>
              <Button size="sm" variant="outline" asChild>
                <Link to="/admin/invoices">Back</Link>
              </Button>
              <Button
                size="sm"
                variant="outline"
                disabled={!invoice || printPdf.isPending}
                onClick={() => {
                  if (!invoice) return;
                  printPdf.mutate(invoice.id, {
                    onSuccess: (path) => toast.success(`A4 PDF saved to ${path}`),
                    onError: (err: Error) => toast.error(err.message),
                  });
                }}
              >
                {printPdf.isPending ? "Generating…" : "Download A4 PDF"}
              </Button>
              <Button
                size="sm"
                variant="brass"
                disabled={!invoice || printReceipt.isPending}
                onClick={() => {
                  if (!invoice) return;
                  printReceipt.mutate(invoice.id, {
                    onSuccess: () => toast.success("Receipt sent to printer"),
                    onError: (err: Error) =>
                      toast.error(`Receipt printing failed: ${err.message}`),
                  });
                }}
              >
                {printReceipt.isPending ? "Printing…" : "Print receipt"}
              </Button>
            </>
          }
    >
      {isLoading && <p className="text-sm text-muted-foreground">Loading invoice…</p>}
      {!isLoading && !invoice && (
        <p className="text-sm text-muted-foreground">This invoice could not be found.</p>
      )}
      {invoice && (
        <div className="mx-auto max-w-3xl rounded-lg border border-border bg-card p-8 print:border-0 print:shadow-none">
          <div className="flex flex-wrap items-start justify-between gap-4 border-b border-border pb-6">
            <div>
              <h2 className="font-display text-3xl">{BUSINESS.name}</h2>
              {BUSINESS.branches.map((b) => (
                <p key={b.label} className="mt-1 text-xs text-muted-foreground">
                  {b.label}: {b.address}
                </p>
              ))}
              <p className="text-xs text-muted-foreground">
                {BUSINESS.phone} · {BUSINESS.email}
              </p>
            </div>
            <div className="text-right">
              <p className="text-xs uppercase tracking-wider text-muted-foreground">Invoice</p>
              <p className="font-display text-2xl">{invoice.invoice_no}</p>
              <p className="text-xs text-muted-foreground">{formatDate(invoice.created_at)}</p>
            </div>
          </div>

          <div className="grid gap-4 py-6 sm:grid-cols-2">
            <div>
              <p className="text-xs uppercase tracking-wider text-muted-foreground">Billed to</p>
              <p className="mt-1 font-medium">{invoice.customer_name}</p>
            </div>
            <div className="sm:text-right">
              <p className="text-xs uppercase tracking-wider text-muted-foreground">Payment</p>
              <p className="mt-1 capitalize">{invoice.payment_method}</p>
              {invoice.delivery_date && (
                <>
                  <p className="mt-3 text-xs uppercase tracking-wider text-muted-foreground">
                    Delivery date
                  </p>
                  <p className="mt-1">{formatDate(invoice.delivery_date)}</p>
                </>
              )}
            </div>
          </div>

          <table className="w-full text-sm">
            <thead className="border-y border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="py-2">Item</th>
                <th className="py-2 text-right">Qty</th>
                {items.some((i: InvoiceItem) => i.total_area != null && i.total_area > 0) && (
                  <th className="py-2 text-right">Area</th>
                )}
                <th className="py-2 text-right">Rate</th>
                <th className="py-2 text-right">Amount</th>
              </tr>
            </thead>
            <tbody>
              {items.map((item: InvoiceItem) => (
                <tr key={item.id} className="border-b border-border">
                  <td className="py-3">{item.product_name}</td>
                  <td className="py-3 text-right">
                    {item.quantity} {item.unit}
                  </td>
                  {items.some((i: InvoiceItem) => i.total_area != null && i.total_area > 0) && (
                    <td className="py-3 text-right">
                      {item.total_area != null && item.total_area > 0 ? `${Number(item.total_area).toFixed(3)} sqm` : ""}
                    </td>
                  )}
                  <td className="py-3 text-right">{currency(item.unit_price)}</td>
                  <td className="py-3 text-right">{currency(item.line_total)}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <div className="ml-auto mt-6 w-full max-w-xs space-y-1 text-sm">
            <div className="flex justify-between text-muted-foreground">
              <span>Subtotal</span>
              <span>{currency(invoice.subtotal)}</span>
            </div>
            <div className="flex justify-between text-muted-foreground">
              <span>Discount</span>
              <span>-{currency(invoice.discount)}</span>
            </div>
            {invoice.previous_balance > 0 && (
              <div className="flex justify-between text-muted-foreground">
                <span>Previous balance</span>
                <span>{currency(invoice.previous_balance)}</span>
              </div>
            )}
            <div className="flex justify-between border-t border-border pt-2 text-lg font-semibold">
              <span>Total</span>
              <span>{currency(invoice.total)}</span>
            </div>
            <div className="flex justify-between text-muted-foreground">
              <span>Paid</span>
              <span>{currency(invoice.amount_paid)}</span>
            </div>
            <div className="flex justify-between font-medium">
              <span>Balance due</span>
              <span>{currency(balance)}</span>
            </div>
          </div>

          {invoice.carried_to_invoice_id && (
            <div className="mt-6 flex items-center gap-2 rounded-md border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800 print:hidden">
              <span>This invoice's balance was carried to</span>
              <Link
                to="/admin/invoices/$invoiceId"
                params={{ invoiceId: invoice.carried_to_invoice_id }}
                className="font-semibold underline underline-offset-2 hover:text-emerald-600"
              >
                {(() => {
                  /* Resolve invoice number from query cache or fallback to ID */
                  const carried = queryClient.getQueryData<{ invoice: Invoice }>(["invoice", invoice.carried_to_invoice_id]);
                  return carried?.invoice.invoice_no ?? invoice.carried_to_invoice_id.slice(0, 8);
                })()}
              </Link>
              <ArrowRight className="size-4" />
            </div>
          )}

          {balance > 0 && !invoice.carried_to_invoice_id && (
            <div className="mt-6 flex flex-wrap items-end gap-3 border-t border-border pt-4 print:hidden">
              <div className="space-y-1">
                <Label htmlFor="record-payment">Record payment — due {currency(balance)}</Label>
                <NumberInput
                  id="record-payment"
                  min={1}
                  max={balance}
                  placeholder={String(balance)}
                  value={paymentAmount}
                  onChange={(e) => setPaymentAmount(e.target.value)}
                  className="w-44"
                />
              </div>
              <Button
                size="sm"
                variant="brass"
                disabled={recordPayment.isPending}
                onClick={() => recordPayment.mutate()}
              >
                {recordPayment.isPending
                  ? "Saving…"
                  : Number(paymentAmount) > 0 && Number(paymentAmount) !== balance
                  ? "Record partial payment"
                  : "Clear full balance"}
              </Button>
            </div>
          )}

          {invoice.notes && (
            <p className="mt-6 border-t border-border pt-4 text-xs text-muted-foreground">
              {invoice.notes}
            </p>
          )}
          <p className="mt-6 text-center text-xs text-muted-foreground">
            Thank you for your business — {BUSINESS.name}
          </p>
        </div>
      )}
    </AdminShell>
  );
}