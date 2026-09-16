import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";
import { Trash2 } from "lucide-react";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { NumberInput } from "@/components/ui/number-input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { currency } from "@/features/inventory/api";
import { formatDate } from "@/features/invoices/api";
import { useSuppliersRealtime } from "@/lib/tauri-events";
import {
  useSuppliers,
  useSupplierPurchases,
  useCreateSupplier,
  useUpdateSupplier,
  useDeleteSupplier,
  useCreateSupplierPurchase,
  useMarkSupplierPurchasePaid,
} from "@/features/suppliers/api";
import type {
  Supplier,
  CreateSupplierInput,
  SupplierPurchase,
  CreateSupplierPurchaseItemInput,
} from "@/lib/api-client";

export const Route = createFileRoute("/_authenticated/admin/suppliers")({
  component: SuppliersPage,
});

type SupplierDraft = {
  name: string;
  phone: string;
  email: string;
  company: string;
  address: string;
  notes: string;
};

const emptySupplierDraft: SupplierDraft = {
  name: "",
  phone: "",
  email: "",
  company: "",
  address: "",
  notes: "",
};

function toSupplierDraft(s: Supplier): SupplierDraft {
  return {
    name: s.name,
    phone: s.phone ?? "",
    email: s.email ?? "",
    company: s.company ?? "",
    address: s.address ?? "",
    notes: s.notes ?? "",
  };
}

function SuppliersPage() {
  const queryClient = useQueryClient();
  useSuppliersRealtime();

  const { data: suppliers = [], isLoading: loadingSuppliers } = useSuppliers();
  const { data: purchases = [], isLoading: loadingPurchases } = useSupplierPurchases();

  // --- Supplier Directory ---
  const [expanded, setExpanded] = useState<string | null>(null);

  // --- Add/Edit Supplier dialog ---
  const [supplierDialogOpen, setSupplierDialogOpen] = useState(false);
  const [editingSupplier, setEditingSupplier] = useState<Supplier | null>(null);
  const [supplierForm, setSupplierForm] = useState<SupplierDraft>(emptySupplierDraft);

  const supplierField = (key: keyof SupplierDraft) => ({
    value: supplierForm[key],
    onChange: (e: { target: { value: string } }) =>
      setSupplierForm((f) => ({ ...f, [key]: e.target.value })),
  });

  const createSupplier = useCreateSupplier();
  const saveNewSupplier = useMutation({
    mutationFn: async () => {
      if (supplierForm.name.trim().length < 2) throw new Error("Supplier name is required");
      await createSupplier.mutateAsync({
        name: supplierForm.name.trim(),
        phone: supplierForm.phone.trim() || undefined,
        email: supplierForm.email.trim() || undefined,
        company: supplierForm.company.trim() || undefined,
        address: supplierForm.address.trim() || undefined,
        notes: supplierForm.notes.trim() || undefined,
      });
    },
    onSuccess: () => {
      toast.success("Supplier added");
      resetSupplierDialog();
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const updateSupplier = useUpdateSupplier();
  const saveEditSupplier = useMutation({
    mutationFn: async () => {
      if (!editingSupplier) throw new Error("No supplier selected");
      if (supplierForm.name.trim().length < 2) throw new Error("Supplier name is required");
      await updateSupplier.mutateAsync({
        id: editingSupplier.id,
        input: {
          name: supplierForm.name.trim(),
          phone: supplierForm.phone.trim() || undefined,
          email: supplierForm.email.trim() || undefined,
          company: supplierForm.company.trim() || undefined,
          address: supplierForm.address.trim() || undefined,
          notes: supplierForm.notes.trim() || undefined,
        },
      });
    },
    onSuccess: () => {
      toast.success("Supplier updated");
      resetSupplierDialog();
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const deleteSupplier = useDeleteSupplier();
  const removeSupplier = useMutation({
    mutationFn: (id: string) => deleteSupplier.mutateAsync(id),
    onSuccess: () => toast.success("Supplier deleted"),
    onError: (err: Error) => toast.error(err.message),
  });

  function resetSupplierDialog() {
    setSupplierForm(emptySupplierDraft);
    setEditingSupplier(null);
    setSupplierDialogOpen(false);
  }

  function openAddSupplier() {
    setEditingSupplier(null);
    setSupplierForm(emptySupplierDraft);
    setSupplierDialogOpen(true);
  }

  function openEditSupplier(s: Supplier) {
    setEditingSupplier(s);
    setSupplierForm(toSupplierDraft(s));
    setSupplierDialogOpen(true);
  }

  // --- Record Purchase dialog ---
  const [purchaseDialogOpen, setPurchaseDialogOpen] = useState(false);
  const [purchaseForm, setPurchaseForm] = useState<{
    supplier_id: string;
    supplier_name: string;
    discount: number;
    amount_paid: string;
    payment_method: "cash" | "bank" | "credit";
    notes: string;
    items: CreateSupplierPurchaseItemInput[];
  }>({
    supplier_id: "",
    supplier_name: "",
    discount: 0,
    amount_paid: "0",
    payment_method: "cash",
    notes: "",
    items: [],
  });

  const purchaseSubtotal = purchaseForm.items.reduce((sum, i) => sum + i.line_total, 0);
  const purchaseTotal = purchaseSubtotal - purchaseForm.discount;

  function addItem() {
    setPurchaseForm((f) => ({
      ...f,
      items: [
        ...f.items,
        { description: "", quantity: 0, unit: "pcs", unit_price: 0, line_total: 0 },
      ],
    }));
  }

  function removeItem(idx: number) {
    setPurchaseForm((f) => ({
      ...f,
      items: f.items.filter((_, i) => i !== idx),
    }));
  }

  function updateItem(
    idx: number,
    field: keyof CreateSupplierPurchaseItemInput,
    value: string | number,
  ) {
    setPurchaseForm((f) => {
      const items = [...f.items];
      items[idx] = { ...items[idx], [field]: value };
      if (field === "quantity" || field === "unit_price") {
        items[idx].line_total = items[idx].quantity * items[idx].unit_price;
      }
      return { ...f, items };
    });
  }

  const createPurchase = useCreateSupplierPurchase();
  const recordPurchase = useMutation({
    mutationFn: async () => {
      if (!purchaseForm.supplier_id) throw new Error("Select a supplier");
      if (purchaseForm.items.length === 0) throw new Error("Add at least one item");
      for (const item of purchaseForm.items) {
        if (!item.description.trim()) throw new Error("Each item needs a description");
        if (item.quantity <= 0) throw new Error(`${item.description}: quantity must be at least 1`);
        if (item.unit_price < 0) throw new Error(`${item.description}: price cannot be negative`);
      }
      if (purchaseForm.discount < 0) throw new Error("Discount cannot be negative");
      if (purchaseTotal < 0) throw new Error("Discount cannot exceed subtotal");
      const amountPaid = Number(purchaseForm.amount_paid) || 0;
      if (amountPaid < 0) throw new Error("Amount paid cannot be negative");
      if (amountPaid > purchaseTotal) throw new Error("Amount paid cannot exceed total");

      await createPurchase.mutateAsync({
        supplier_id: purchaseForm.supplier_id,
        supplier_name: purchaseForm.supplier_name,
        subtotal: purchaseSubtotal,
        discount: purchaseForm.discount,
        total: purchaseTotal,
        amount_paid: amountPaid,
        payment_method: purchaseForm.payment_method,
        notes: purchaseForm.notes.trim() || undefined,
        items: purchaseForm.items.map((i) => ({
          ...i,
          description: i.description.trim(),
        })),
      });
    },
    onSuccess: () => {
      toast.success("Purchase recorded");
      resetPurchaseDialog();
    },
    onError: (err: Error) => toast.error(err.message),
  });

  function resetPurchaseDialog() {
    setPurchaseForm({
      supplier_id: "",
      supplier_name: "",
      discount: 0,
      amount_paid: "0",
      payment_method: "cash",
      notes: "",
      items: [],
    });
    setPurchaseDialogOpen(false);
  }

  // --- Purchase Payment ---
  const [paymentPurchaseId, setPaymentPurchaseId] = useState<string | null>(null);
  const [paymentAmount, setPaymentAmount] = useState("");

  const markPaid = useMarkSupplierPurchasePaid();
  const recordPayment = useMutation({
    mutationFn: ({ purchase, amount }: { purchase: SupplierPurchase; amount: number }) =>
      markPaid.mutateAsync({
        id: purchase.id,
        amount,
        payment_method: purchase.payment_method,
      }),
    onSuccess: (_data, vars) => {
      const balance = Number(vars.purchase.total) - Number(vars.purchase.amount_paid);
      const remaining = Math.max(0, balance - vars.amount);
      toast.success(
        remaining > 0
          ? `Payment recorded — ${currency(remaining)} still due`
          : "Purchase fully paid",
      );
      setPaymentPurchaseId(null);
      setPaymentAmount("");
    },
    onError: (err: Error) => toast.error(err.message),
  });

  return (
    <AdminShell
      title="Suppliers"
      actions={
        <>
          <Dialog
            open={purchaseDialogOpen}
            onOpenChange={(v) => {
              setPurchaseDialogOpen(v);
              if (!v) resetPurchaseDialog();
            }}
          >
            <DialogTrigger asChild>
              <Button size="sm" variant="brass">
                Record Purchase
              </Button>
            </DialogTrigger>
            <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
              <DialogHeader>
                <DialogTitle>Record Purchase</DialogTitle>
              </DialogHeader>
              <div className="space-y-4">
                {/* Supplier select */}
                <div className="space-y-2">
                  <Label>Supplier</Label>
                  <select
                    className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                    value={purchaseForm.supplier_id}
                    onChange={(e) => {
                      const id = e.target.value;
                      const s = suppliers.find((s) => s.id === id);
                      setPurchaseForm((f) => ({
                        ...f,
                        supplier_id: id,
                        supplier_name: s?.name ?? "",
                      }));
                    }}
                  >
                    <option value="">Select supplier…</option>
                    {suppliers.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.name}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Line items */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>Items</Label>
                    <Button size="sm" variant="outline" onClick={addItem}>
                      Add item
                    </Button>
                  </div>
                  {purchaseForm.items.length === 0 && (
                    <p className="text-xs text-muted-foreground">
                      No items added yet. Click "Add item" to start.
                    </p>
                  )}
                  {purchaseForm.items.map((item, idx) => (
                    <div
                      key={idx}
                      className="grid grid-cols-[1fr_60px_60px_80px_80px_32px] items-end gap-2 rounded-md border border-border p-2"
                    >
                      <div className="space-y-1">
                        <Label className="text-xs">Description</Label>
                        <Input
                          value={item.description}
                          onChange={(e) => updateItem(idx, "description", e.target.value)}
                          placeholder="Item name"
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs">Qty</Label>
                        <NumberInput
                          min={1}
                          value={String(item.quantity)}
                          onChange={(e) =>
                            updateItem(idx, "quantity", Math.max(1, Number(e.target.value) || 1))
                          }
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs">Unit</Label>
                        <Input
                          value={item.unit}
                          onChange={(e) => updateItem(idx, "unit", e.target.value)}
                          placeholder="pcs"
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs">Unit Price</Label>
                        <NumberInput
                          min={0}
                          value={String(item.unit_price)}
                          onChange={(e) =>
                            updateItem(idx, "unit_price", Math.max(0, Number(e.target.value) || 0))
                          }
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs">Total</Label>
                        <p className="h-9 flex items-center text-sm font-medium">
                          {currency(item.line_total)}
                        </p>
                      </div>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-9 w-9 p-0"
                        onClick={() => removeItem(idx)}
                      >
                        <Trash2 className="size-4 text-destructive" />
                      </Button>
                    </div>
                  ))}
                </div>

                {/* Totals */}
                <div className="grid grid-cols-2 gap-4">
                  <div />
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm text-muted-foreground">
                      <span>Subtotal</span>
                      <span>{currency(purchaseSubtotal)}</span>
                    </div>
                    <div className="flex items-center justify-between text-sm">
                      <Label>Discount</Label>
                      <NumberInput
                        min={0}
                        className="w-28"
                        value={String(purchaseForm.discount)}
                        onChange={(e) =>
                          setPurchaseForm((f) => ({
                            ...f,
                            discount: Math.max(0, Number(e.target.value) || 0),
                          }))
                        }
                      />
                    </div>
                    <div className="flex justify-between border-t border-border pt-2 text-lg font-semibold">
                      <span>Total</span>
                      <span>{currency(purchaseTotal)}</span>
                    </div>
                  </div>
                </div>

                {/* Payment */}
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label>Amount Paid</Label>
                    <NumberInput
                      min={0}
                      placeholder="0"
                      value={purchaseForm.amount_paid}
                      onChange={(e) =>
                        setPurchaseForm((f) => ({ ...f, amount_paid: e.target.value }))
                      }
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>Payment Method</Label>
                    <div className="grid grid-cols-3 gap-2">
                      {(["cash", "bank", "credit"] as const).map((m) => (
                        <button
                          key={m}
                          type="button"
                          onClick={() => setPurchaseForm((f) => ({ ...f, payment_method: m }))}
                          className={`rounded-md border px-3 py-2 text-sm capitalize ${
                            purchaseForm.payment_method === m
                              ? "border-foreground bg-foreground text-background"
                              : "border-border"
                          }`}
                        >
                          {m}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                {/* Notes */}
                <div className="space-y-2">
                  <Label>Notes</Label>
                  <Textarea
                    value={purchaseForm.notes}
                    onChange={(e) =>
                      setPurchaseForm((f) => ({ ...f, notes: e.target.value }))
                    }
                    placeholder="Optional notes"
                    rows={2}
                  />
                </div>

                <Button
                  variant="brass"
                  className="w-full"
                  disabled={recordPurchase.isPending}
                  onClick={() => recordPurchase.mutate()}
                >
                  {recordPurchase.isPending ? "Saving…" : "Record purchase"}
                </Button>
              </div>
            </DialogContent>
          </Dialog>

          <Dialog
            open={supplierDialogOpen}
            onOpenChange={(v) => {
              setSupplierDialogOpen(v);
              if (!v) resetSupplierDialog();
            }}
          >
            <DialogTrigger asChild>
              <Button size="sm" variant="outline" onClick={openAddSupplier}>
                Add supplier
              </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>
                  {editingSupplier ? "Edit supplier" : "New supplier"}
                </DialogTitle>
              </DialogHeader>
              <div className="space-y-4">
                {(["name", "phone", "email", "company", "address"] as const).map((key) => (
                  <div key={key} className="space-y-2">
                    <Label className="capitalize">{key}{key === "name" ? " *" : ""}</Label>
                    <Input {...supplierField(key)} maxLength={200} />
                  </div>
                ))}
                <div className="space-y-2">
                  <Label>Notes</Label>
                  <Textarea
                    value={supplierForm.notes}
                    onChange={(e) =>
                      setSupplierForm((f) => ({ ...f, notes: e.target.value }))
                    }
                    placeholder="Optional notes"
                    rows={2}
                  />
                </div>
                <Button
                  variant="brass"
                  className="w-full"
                  disabled={
                    editingSupplier ? saveEditSupplier.isPending : saveNewSupplier.isPending
                  }
                  onClick={() =>
                    editingSupplier ? saveEditSupplier.mutate() : saveNewSupplier.mutate()
                  }
                >
                  {editingSupplier
                    ? saveEditSupplier.isPending
                      ? "Saving…"
                      : "Save changes"
                    : saveNewSupplier.isPending
                      ? "Saving…"
                      : "Add supplier"}
                </Button>
              </div>
            </DialogContent>
          </Dialog>
        </>
      }
    >
      {/* Section 1: Supplier Directory */}
      <div className="mb-8 space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Supplier Directory
        </h2>
        <div className="overflow-x-auto rounded-lg border border-border bg-card">
          <table className="w-full min-w-[680px] text-sm">
            <thead className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="px-4 py-3">Name</th>
                <th className="px-4 py-3">Company</th>
                <th className="px-4 py-3">Phone</th>
                <th className="px-4 py-3">Email</th>
                <th className="px-4 py-3 text-right">Balance Owed</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {loadingSuppliers && (
                <tr>
                  <td colSpan={6} className="px-4 py-6 text-muted-foreground">
                    Loading suppliers…
                  </td>
                </tr>
              )}
              {!loadingSuppliers && suppliers.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-4 py-6 text-muted-foreground">
                    No suppliers recorded yet.
                  </td>
                </tr>
              )}
              {suppliers.map((s) => {
                const supplierPurchases = purchases.filter((p) => p.supplier_id === s.id);
                return (
                  <>
                    <tr
                      key={s.id}
                      onClick={() => setExpanded(expanded === s.id ? null : s.id)}
                      className="cursor-pointer border-b border-border last:border-0 hover:bg-accent/50"
                    >
                      <td className="px-4 py-3 font-medium">{s.name}</td>
                      <td className="px-4 py-3 text-muted-foreground">{s.company ?? "—"}</td>
                      <td className="px-4 py-3 text-muted-foreground">{s.phone ?? "—"}</td>
                      <td className="px-4 py-3 text-muted-foreground">{s.email ?? "—"}</td>
                      <td
                        className={`px-4 py-3 text-right ${
                          Number(s.outstanding_balance) > 0 ? "text-destructive" : ""
                        }`}
                      >
                        {currency(s.outstanding_balance)}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <div className="flex items-center justify-end gap-1" onClick={(e) => e.stopPropagation()}>
                          <Button
                            size="sm"
                            variant="outline"
                            onClick={() => openEditSupplier(s)}
                          >
                            Edit
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            disabled={removeSupplier.isPending}
                            onClick={() => {
                              if (confirm(`Delete "${s.name}"?`)) removeSupplier.mutate(s.id);
                            }}
                          >
                            <Trash2 className="size-4 text-destructive" />
                          </Button>
                        </div>
                      </td>
                    </tr>
                    {expanded === s.id && (
                      <tr key={`${s.id}-history`} className="border-b border-border bg-muted/40">
                        <td colSpan={6} className="px-4 py-3">
                          {supplierPurchases.length === 0 ? (
                            <p className="text-xs text-muted-foreground">No purchases yet.</p>
                          ) : (
                            <ul className="space-y-1 text-xs">
                              {supplierPurchases.map((p) => (
                                <li key={p.id} className="flex justify-between">
                                  <span>
                                    {p.purchase_no} · {formatDate(p.created_at)}
                                  </span>
                                  <span>{currency(p.total)}</span>
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
      </div>

      {/* Section 3: Purchase History */}
      <div className="space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          Purchase History
        </h2>
        <div className="overflow-x-auto rounded-lg border border-border bg-card">
          <table className="w-full min-w-[720px] text-sm">
            <thead className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="px-4 py-3">PO #</th>
                <th className="px-4 py-3">Supplier</th>
                <th className="px-4 py-3 text-right">Total</th>
                <th className="px-4 py-3 text-right">Paid</th>
                <th className="px-4 py-3 text-right">Balance</th>
                <th className="px-4 py-3">Date</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {loadingPurchases && (
                <tr>
                  <td colSpan={7} className="px-4 py-6 text-muted-foreground">
                    Loading purchases…
                  </td>
                </tr>
              )}
              {!loadingPurchases && purchases.length === 0 && (
                <tr>
                  <td colSpan={7} className="px-4 py-6 text-muted-foreground">
                    No purchases recorded yet.
                  </td>
                </tr>
              )}
              {purchases.map((p) => {
                const balance = Math.max(0, Number(p.total) - Number(p.amount_paid));
                return (
                  <tr key={p.id} className="border-b border-border last:border-0">
                    <td className="px-4 py-3 font-medium">{p.purchase_no}</td>
                    <td className="px-4 py-3">{p.supplier_name}</td>
                    <td className="px-4 py-3 text-right">{currency(p.total)}</td>
                    <td className="px-4 py-3 text-right">{currency(p.amount_paid)}</td>
                    <td
                      className={`px-4 py-3 text-right ${
                        balance > 0 ? "text-destructive" : ""
                      }`}
                    >
                      {currency(balance)}
                    </td>
                    <td className="px-4 py-3 text-muted-foreground">
                      {formatDate(p.created_at)}
                    </td>
                    <td className="px-4 py-3 text-right">
                      {balance > 0 && (
                        <>
                          {paymentPurchaseId === p.id ? (
                            <div className="flex items-end justify-end gap-2">
                              <NumberInput
                                min={1}
                                max={balance}
                                placeholder={String(balance)}
                                value={paymentAmount}
                                onChange={(e) => setPaymentAmount(e.target.value)}
                                className="w-28"
                              />
                              <Button
                                size="sm"
                                variant="brass"
                                disabled={recordPayment.isPending}
                                onClick={() => {
                                  const amt =
                                    Number(paymentAmount) > 0
                                      ? Number(paymentAmount)
                                      : balance;
                                  recordPayment.mutate({ purchase: p, amount: amt });
                                }}
                              >
                                {recordPayment.isPending ? "…" : "Save"}
                              </Button>
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => {
                                  setPaymentPurchaseId(null);
                                  setPaymentAmount("");
                                }}
                              >
                                Cancel
                              </Button>
                            </div>
                          ) : (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => setPaymentPurchaseId(p.id)}
                            >
                              Record Payment
                            </Button>
                          )}
                        </>
                      )}
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
