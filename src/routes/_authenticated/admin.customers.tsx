import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { currency } from "@/features/inventory/api";
import { useCustomers, useCreateCustomer, useReconcileBalances, type Customer } from "@/features/customers/api";
import { useInvoices } from "@/features/invoices/api";
import { formatDate } from "@/features/invoices/api";

export const Route = createFileRoute("/_authenticated/admin/customers")({
  component: CustomersPage,
});

function CustomersPage() {
  const queryClient = useQueryClient();
  const { data: customers = [] } = useCustomers();
  const { data: invoices = [] } = useInvoices();
  const createCustomer = useCreateCustomer();
  const [open, setOpen] = useState(false);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [form, setForm] = useState({ name: "", phone: "", email: "", address: "" });
  const reconcile = useReconcileBalances();

  const create = useMutation({
    mutationFn: async () => {
      if (form.name.trim().length < 2) throw new Error("Customer name is required");
      await createCustomer.mutateAsync({
        name: form.name.trim(),
        phone: form.phone.trim() || null,
        email: form.email.trim() || null,
        address: form.address.trim() || null,
      });
    },
    onSuccess: () => {
      toast.success("Customer added");
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      setForm({ name: "", phone: "", email: "", address: "" });
      setOpen(false);
    },
    onError: (err: Error) => toast.error(err.message),
  });

  return (
    <AdminShell
      title="Customers"
      actions={
        <>
          <Button
            size="sm"
            variant="outline"
            disabled={reconcile.isPending}
            onClick={() => reconcile.mutate(undefined, {
              onSuccess: (n) => toast.success(`${n} customer balances reconciled`),
              onError: (err: Error) => toast.error(err.message),
            })}
          >
            Reconcile
          </Button>
          <Dialog open={open} onOpenChange={setOpen}>
          <DialogTrigger asChild>
            <Button size="sm" variant="brass">
              Add customer
            </Button>
          </DialogTrigger>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>New customer</DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
              {(["name", "phone", "email", "address"] as const).map((key) => (
                <div key={key} className="space-y-2">
                  <Label className="capitalize">{key}</Label>
                  <Input
                    value={form[key]}
                    maxLength={200}
                    onChange={(e) => setForm((f) => ({ ...f, [key]: e.target.value }))}
                  />
                </div>
              ))}
              <Button
                variant="brass"
                className="w-full"
                disabled={create.isPending}
                onClick={() => create.mutate()}
              >
                Save customer
              </Button>
            </div>
          </DialogContent>
        </Dialog>
        </>
      }
    >
      <div className="overflow-x-auto rounded-lg border border-border bg-card">
        <table className="w-full min-w-[680px] text-sm">
          <thead className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
            <tr>
              <th className="px-4 py-3">Customer</th>
              <th className="px-4 py-3">Contact</th>
              <th className="px-4 py-3 text-right">Outstanding</th>
              <th className="px-4 py-3 text-right">Orders</th>
            </tr>
          </thead>
          <tbody>
            {customers.length === 0 && (
              <tr>
                <td colSpan={4} className="px-4 py-6 text-muted-foreground">
                  No customers recorded yet.
                </td>
              </tr>
            )}
            {customers.map((c: Customer) => {
              const history = invoices.filter((i) => i.customer_id === c.id);
              return (
                <>
                  <tr
                    key={c.id}
                    onClick={() => setExpanded(expanded === c.id ? null : c.id)}
                    className="cursor-pointer border-b border-border last:border-0 hover:bg-accent/50"
                  >
                    <td className="px-4 py-3 font-medium">{c.name}</td>
                    <td className="px-4 py-3 text-muted-foreground">
                      {c.phone ?? c.email ?? "—"}
                    </td>
                    <td
                      className={`px-4 py-3 text-right ${
                        Number(c.outstanding_balance) > 0 ? "text-destructive" : ""
                      }`}
                    >
                      {currency(c.outstanding_balance)}
                    </td>
                    <td className="px-4 py-3 text-right">{history.length}</td>
                  </tr>
                  {expanded === c.id && (
                    <tr key={`${c.id}-history`} className="border-b border-border bg-muted/40">
                      <td colSpan={4} className="px-4 py-3">
                        {history.length === 0 ? (
                          <p className="text-xs text-muted-foreground">No invoices yet.</p>
                        ) : (
                          <ul className="space-y-1 text-xs">
                            {history.map((i) => (
                              <li key={i.id} className="flex justify-between">
                                <span>
                                  {i.invoice_no} · {formatDate(i.created_at)}
                                </span>
                                <span>{currency(i.total)}</span>
                              </li>
                            ))}
                          </ul>
                        )}
                      </td>
                    </tr>
                  )}
                </>
              );
            })}
          </tbody>
        </table>
      </div>
    </AdminShell>
  );
}