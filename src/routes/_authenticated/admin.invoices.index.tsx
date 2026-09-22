import { createFileRoute, Link } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Printer, CheckCircle2, Search, X, ArrowRight } from "lucide-react";
import { toast } from "sonner";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

import { currency } from "@/features/inventory/api";
import {
  useInvoices,
  useInvoiceSearch,
  useMarkInvoicePaid,
  usePrintReceipt,
  type Invoice,
} from "@/features/invoices/api";
import { formatDate } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/invoices/")({
  component: InvoicesPage,
});

function InvoicesPage() {
  const queryClient = useQueryClient();
  const [searchInput, setSearchInput] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");

  useEffect(() => {
    const t = setTimeout(() => setDebouncedQuery(searchInput), 300);
    return () => clearTimeout(t);
  }, [searchInput]);

  const searching = debouncedQuery.trim().length > 0;

  const { data: recentInvoices = [], isLoading } = useInvoices();
  const search = useInvoiceSearch(debouncedQuery);

  const invoices = searching ? (search.data ?? []) : recentInvoices;
  const isSearchingNow = searching && search.isFetching;

  const markPaid = useMarkInvoicePaid();
  const printReceipt = usePrintReceipt();

  const printInvoice = (id: string) =>
    printReceipt.mutate(id, {
      onSuccess: () => toast.success("Receipt sent to printer"),
      onError: (err: Error) => toast.error(`Receipt printing failed: ${err.message}`),
    });

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
        (old ?? []).map((i) => (i.id === inv.id ? { ...i, amount_paid: Number(i.total) } : i)),
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

  return (
    <AdminShell
      title="Invoices"
      actions={
        <Button asChild size="sm" variant="brass">
          <Link to="/admin/pos">New sale</Link>
        </Button>
      }
    >
      <div className="space-y-3">
        <div className="relative">
          <Search className="absolute left-3 top-2.5 size-4 text-muted-foreground" />
          <Input
            className="pl-9 pr-9"
            placeholder="Search all invoices by number or customer"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
          />
          {searchInput.length > 0 && (
            <button
              type="button"
              aria-label="Clear search"
              onClick={() => setSearchInput("")}
              className="absolute right-2 top-1.5 rounded p-1 text-muted-foreground hover:text-foreground"
            >
              <X className="size-4" />
            </button>
          )}
        </div>
        {searching && (
          <p className="text-xs text-muted-foreground">
            {isSearchingNow
              ? "Searching all invoices…"
              : `${invoices.length} ${invoices.length === 1 ? "match" : "matches"} across all invoices for "${debouncedQuery.trim()}"`}
          </p>
        )}
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
              {(isLoading || isSearchingNow) && (
                <tr>
                  <td colSpan={7} className="px-4 py-6 text-muted-foreground">
                    {isSearchingNow ? "Searching all invoices…" : "Loading invoices…"}
                  </td>
                </tr>
              )}
              {!isLoading && !isSearchingNow && invoices.length === 0 && (
                <tr>
                  <td colSpan={7} className="px-4 py-6 text-muted-foreground">
                    {searching
                      ? "No invoices match your search."
                      : "No invoices yet — create one from the Point of Sale screen."}
                  </td>
                </tr>
              )}
              {invoices.map((inv: Invoice) => {
                const balance = Math.max(0, Number(inv.total) - Number(inv.amount_paid));
                const carried = !!inv.carried_to_invoice_id;
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
                    <td className="px-4 py-3 text-muted-foreground">
                      {formatDate(inv.created_at)}
                    </td>
                    <td className="px-4 py-3 capitalize text-muted-foreground">
                      {inv.payment_method}
                    </td>
                    <td className="px-4 py-3 text-right">{currency(inv.total)}</td>
                    <td className="px-4 py-3 text-right">
                      {carried ? (
                        <Link
                          to="/admin/invoices/$invoiceId"
                          params={{ invoiceId: inv.carried_to_invoice_id! }}
                          className="inline-flex items-center gap-1 text-xs font-medium text-emerald-600 hover:underline"
                        >
                          Carried
                          <ArrowRight className="size-3" />
                        </Link>
                      ) : (
                        <span className={balance > 0 ? "text-destructive" : "text-muted-foreground"}>
                          {currency(balance)}
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex justify-end gap-2">
                        {!carried && balance > 0 && (
                          <Button
                            size="sm"
                            variant="brass"
                            disabled={clearPayment.isPending}
                            onClick={() => clearPayment.mutate(inv)}
                          >
                            <CheckCircle2 className="size-4" /> Payment cleared
                          </Button>
                        )}
                        <Button
                          size="sm"
                          variant="outline"
                          disabled={printReceipt.isPending}
                          onClick={() => printInvoice(inv.id)}
                        >
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
      </div>
    </AdminShell>
  );
}
