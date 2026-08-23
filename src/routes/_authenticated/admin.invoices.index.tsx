import { createFileRoute, Link } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Printer, CheckCircle2 } from "lucide-react";
import { toast } from "sonner";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { useInvoiceRealtime } from "@/lib/tauri-events";
import { currency } from "@/features/inventory/api";
import { useInvoices, useMarkInvoicePaid, type Invoice } from "@/features/invoices/api";
import { formatDate } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/invoices/")({
  component: InvoicesPage,
});

function InvoicesPage() {
  const queryClient = useQueryClient();
  useInvoiceRealtime();
  const { data: invoices = [], isLoading } = useInvoices();

  const markPaid = useMarkInvoicePaid();

  const clearPayment = useMutation({
    mutationFn: (inv: Invoice) =>
      markPaid.mutateAsync({
        id: inv.id,
        amount: Math.max(0, Number(inv.total) - Number(inv.amount_paid)),
        payment_method: inv.payment_method,
      }),
    onMutate: async (inv: Invoice) => {
      await queryClient.cancelQueries({ queryKey: ["invoices"] });
      const previous = queryClient.getQueryData<Invoice[]>(["invoices"]);
      queryClient.setQueryData<Invoice[]>(["invoices"], (old) =>
        (old ?? []).map((i) =>
          i.id === inv.id ? { ...i, amount_paid: Number(i.total) } : i,
        ),
      );
      return { previous };
    },
    onError: (err: Error, _id, context) => {
      if (context?.previous) queryClient.setQueryData(["invoices"], context.previous);
      toast.error(err.message);
    },
    onSuccess: () => {
      toast.success("Payment marked as cleared");
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["invoices"] });
      queryClient.invalidateQueries({ queryKey: ["customers"] });
    },
  });

  const printInvoice = (id: string) =>
    window.open(`/admin/invoices/${id}?print=1`, "_blank", "noopener");

  return (
    <AdminShell
      title="Invoices"
      actions={
        <Button asChild size="sm" variant="brass">
          <Link to="/admin/pos">New sale</Link>
        </Button>
      }
    >
      <div className="overflow-x-auto rounded-lg border border-border bg-card">
        <table className="w-full min-w-[720px] text-sm">
          <thead className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
            <tr>
              <th className="px-4 py-3">Invoice</th>
              <th className="px-4 py-3">Customer</th>
              <th className="px-4 py-3">Date</th>
              <th className="px-4 py-3">Payment</th>
              <th className="px-4 py-3 text-right">Total</th>
              <th className="px-4 py-3 text-right">Balance</th>
              <th className="px-4 py-3 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {isLoading && (
              <tr>
                <td colSpan={7} className="px-4 py-6 text-muted-foreground">
                  Loading invoices…
                </td>
              </tr>
            )}
            {!isLoading && invoices.length === 0 && (
              <tr>
                <td colSpan={7} className="px-4 py-6 text-muted-foreground">
                  No invoices yet — create one from the Point of Sale screen.
                </td>
              </tr>
            )}
            {invoices.map((inv: Invoice) => {
              const balance = Math.max(0, Number(inv.total) - Number(inv.amount_paid));
              return (
              <tr key={inv.id} className="border-b border-border last:border-0">
                <td className="px-4 py-3">
                  <Link
                    to="/admin/invoices/$invoiceId"
                    params={{ invoiceId: inv.id }}
                    className="font-medium hover:underline"
                  >
                    {inv.invoice_no}
                  </Link>
                </td>
                <td className="px-4 py-3 text-muted-foreground">{inv.customer_name}</td>
                <td className="px-4 py-3 text-muted-foreground">{formatDate(inv.created_at)}</td>
                <td className="px-4 py-3 capitalize text-muted-foreground">
                  {inv.payment_method}
                </td>
                <td className="px-4 py-3 text-right">{currency(inv.total)}</td>
                <td
                  className={`px-4 py-3 text-right ${balance > 0 ? "text-destructive" : "text-muted-foreground"}`}
                >
                  {currency(balance)}
                </td>
                <td className="px-4 py-3">
                  <div className="flex justify-end gap-2">
                    {balance > 0 && (
                      <Button
                        size="sm"
                        variant="brass"
                        disabled={clearPayment.isPending}
                        onClick={() => clearPayment.mutate(inv)}
                      >
                        <CheckCircle2 className="size-4" /> Payment cleared
                      </Button>
                    )}
                    <Button size="sm" variant="outline" onClick={() => printInvoice(inv.id)}>
                      <Printer className="size-4" /> Print
                    </Button>
                  </div>
                </td>
              </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </AdminShell>
  );
}