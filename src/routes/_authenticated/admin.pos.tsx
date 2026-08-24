import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Trash2, Search, Hash } from "lucide-react";
import { toast } from "sonner";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { NumberInput } from "@/components/ui/number-input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { currency, type Product } from "@/features/inventory/api";
import { useCustomersForPOS, type Customer } from "@/features/pos/api";
import { useCreateInvoice, usePrintReceipt } from "@/features/invoices/api";
import { useCreateCustomer } from "@/features/customers/api";
import { useProductsForPOS } from "@/features/pos/api";

export const Route = createFileRoute("/_authenticated/admin/pos")({
  component: PosPage,
});

type Line = { product: Product; qty: number };

const perCarton = (p: Product) =>
  p.category === "tiles" && Number(p.pieces_per_carton) > 0 ? Number(p.pieces_per_carton) : 0;

function PosPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data: products = [] } = useProductsForPOS();
  const { data: customers = [] } = useCustomersForPOS();
  const createInvoice = useCreateInvoice();
  const createCustomer = useCreateCustomer();
  const printReceipt = usePrintReceipt();

  const [query, setQuery] = useState("");
  const [scan, setScan] = useState("");
  const scanRef = useRef<HTMLInputElement>(null);
  const lastScan = useRef<{ code: string; at: number }>({ code: "", at: 0 });
  const [lines, setLines] = useState<Line[]>([]);
  const [customerId, setCustomerId] = useState("");
  const [walkIn, setWalkIn] = useState("Walk-in customer");
  const [walkInPhone, setWalkInPhone] = useState("");
  const [saveWalkIn, setSaveWalkIn] = useState(true);
  const [deliveryDate, setDeliveryDate] = useState("");
  const [discount, setDiscount] = useState("0");
  const [amountPaid, setAmountPaid] = useState("0");
  const [method, setMethod] = useState<"cash" | "bank" | "credit">("cash");
  const [notes, setNotes] = useState("");

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return products.slice(0, 12);
    return products
      .filter((p) =>
        [p.name, p.sku, p.color, p.size, p.finish]
          .filter(Boolean)
          .some((v) => String(v).toLowerCase().includes(q)),
      )
      .slice(0, 12);
  }, [products, query]);

  const subtotal = lines.reduce((acc, l) => acc + Number(l.product.price) * l.qty, 0);
  // A stray minus can never ADD money — discount is clamped to >= 0 and the
  // bill floors at zero.
  const discountValue = Math.max(0, Number(discount) || 0);
  const total = Math.max(0, subtotal - discountValue);
  const paidNow = Math.min(Math.max(0, Number(amountPaid) || 0), total);
  const balanceDue = total - paidNow;

  function addLine(product: Product) {
    setLines((prev) => {
      const found = prev.find((l) => l.product.id === product.id);
      const box = perCarton(product);
      if (found)
        return prev.map((l) =>
          l.product.id === product.id ? { ...l, qty: l.qty + (box || 1) } : l,
        );
      return [...prev, { product, qty: box || 1 }];
    });
  }

  const codeMatches = useMemo(() => {
    const q = scan.trim().toLowerCase();
    if (!q) return [];
    return products
      .filter((p) =>
        [p.sku, p.name].filter(Boolean).some((v) => String(v).toLowerCase().includes(q)),
      )
      .slice(0, 6);
  }, [products, scan]);

  const handleScan = useCallback(
    (raw: string) => {
      const code = raw.trim();
      if (!code) return;
      const now = Date.now();
      if (lastScan.current.code === code && now - lastScan.current.at < 1200) return;
      lastScan.current = { code, at: now };

      const match =
        products.find((p) => p.sku && p.sku.toLowerCase() === code.toLowerCase()) ??
        products.find((p) => p.name.toLowerCase() === code.toLowerCase());
      if (!match) {
        toast.error(`No product with code ${code}`);
        return;
      }
      addLine(match);
      toast.success(`${match.name} added`);
    },
    [products],
  );

  useEffect(() => {
    scanRef.current?.focus();
  }, []);

  const checkout = useMutation({
    mutationFn: async () => {
      if (lines.length === 0) throw new Error("Add at least one product");
      if ((Number(amountPaid) || 0) > total) {
        throw new Error("Amount paid cannot exceed the total");
      }

      // Stock guard (client pre-check; the backend re-validates in its
      // transaction). Aggregates per product across duplicate cart lines.
      const demand = new Map<string, { wanted: number; stock: number; name: string }>();
      for (const l of lines) {
        const entry = demand.get(l.product.id);
        demand.set(l.product.id, {
          wanted: (entry?.wanted ?? 0) + l.qty,
          stock: Number(l.product.stock_qty),
          name: l.product.name,
        });
      }
      const shortfalls = [...demand.values()]
        .filter((d) => d.wanted > d.stock)
        .map((d) => `"${d.name}" — in stock: ${d.stock}, tried: ${d.wanted}`);
      if (shortfalls.length > 0) {
        throw new Error(`Not enough stock: ${shortfalls.join("; ")}`);
      }

      let customer: Customer | null = null;
      if (customerId) {
        customer = customers.find((c) => c.id === customerId) ?? null;
      }

      const walkInName = walkIn.trim();
      if (!customer && saveWalkIn && walkInName && walkInName.toLowerCase() !== "walk-in customer") {
        const existing = customers.find(
          (c) => c.name.trim().toLowerCase() === walkInName.toLowerCase(),
        );
        if (existing) {
          customer = existing;
        } else {
          const created = await createCustomer.mutateAsync({ name: walkInName, phone: walkInPhone.trim() || null });
          customer = created;
        }
      }

      const invoice = await createInvoice.mutateAsync({
        customer_id: customer?.id ?? null,
        customer_name: customer?.name ?? walkInName ?? "Walk-in customer",
        items: lines.map((l) => ({
          product_id: l.product.id,
          product_name: l.product.name,
          quantity: l.qty,
          unit: l.product.unit,
          unit_price: Number(l.product.price),
          line_total: Number(l.product.price) * l.qty,
        })),
        subtotal,
        discount: discountValue,
        total,
        amount_paid: paidNow,
        payment_method: method,
        notes: notes.trim() || null,
        delivery_date: deliveryDate || null,
      });

      return invoice.id as string;
    },
    onSuccess: (invoiceId) => {
      toast.success("Invoice created");
      printReceipt.mutate(invoiceId, {
        onSuccess: () => toast.success("Receipt sent to printer"),
        onError: (err: Error) => toast.error(`Receipt printing failed: ${err.message}`),
      });
      queryClient.invalidateQueries({ queryKey: ["invoices"] });
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      queryClient.invalidateQueries({ queryKey: ["products"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      navigate({ to: "/admin/invoices/$invoiceId", params: { invoiceId } });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  return (
    <AdminShell title="Point of Sale">
      <div className="grid gap-5 lg:grid-cols-[1.2fr_1fr]">
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="mb-4 rounded-md border border-brass/40 bg-brass/5 p-3">
            <div className="flex flex-col gap-2 sm:flex-row">
              <div className="relative flex-1">
                <Hash className="absolute left-3 top-2.5 size-4 text-brass" />
                <Input
                  ref={scanRef}
                  className="pl-9"
                  placeholder="Type tile code (e.g. 100B) and press Enter"
                  value={scan}
                  onChange={(e) => setScan(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      const first = codeMatches[0];
                      const exact = products.find(
                        (p) => (p.sku ?? "").toLowerCase() === scan.trim().toLowerCase(),
                      );
                      if (!exact && first) {
                        addLine(first);
                        toast.success(`${first.sku ?? first.name} added`);
                      } else {
                        handleScan(scan);
                      }
                      setScan("");
                    }
                  }}
                />
                {codeMatches.length > 0 && (
                  <ul className="absolute z-20 mt-1 max-h-64 w-full overflow-y-auto rounded-md border border-border bg-card shadow-lg">
                    {codeMatches.map((p) => (
                      <li key={p.id}>
                        <button
                          type="button"
                          className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm hover:bg-accent"
                          onClick={() => {
                            addLine(p);
                            toast.success(`${p.sku ?? p.name} added`);
                            setScan("");
                            scanRef.current?.focus();
                          }}
                        >
                          <span>
                            <span className="font-medium text-brass">{p.sku ?? "—"}</span>{" "}
                            <span className="text-muted-foreground">{p.name}</span>
                          </span>
                          <span className="shrink-0 text-xs text-muted-foreground">
                            {currency(p.price)} · {p.stock_qty} left
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <Button
                variant="stone"
                className="sm:w-auto"
                onClick={() => {
                  handleScan(scan);
                  setScan("");
                }}
              >
                Add code
              </Button>
            </div>
            <p className="mt-2 text-xs text-muted-foreground">
              Tiles are identified by their stock code — enter the code to add the item instantly.
            </p>
          </div>
          <div className="relative">
            <Search className="absolute left-3 top-2.5 size-4 text-muted-foreground" />
            <Input
              className="pl-9"
              placeholder="Search by code, name, colour or size"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <div className="mt-4 grid gap-2 sm:grid-cols-2">
            {results.map((p) => (
              <button
                key={p.id}
                onClick={() => addLine(p)}
                className="rounded-md border border-border p-3 text-left transition-colors hover:border-foreground"
              >
                <p className="text-sm font-medium">
                  {p.sku ? <span className="text-brass">{p.sku}</span> : null}
                  {p.sku && p.sku !== p.name ? ` · ${p.name}` : p.sku ? "" : p.name}
                </p>
                <p className="text-xs text-muted-foreground">
                  {[p.color, p.size, p.finish].filter(Boolean).join(" · ") || p.unit}
                </p>
                {perCarton(p) > 0 && (
                  <p className="text-xs text-brass">{perCarton(p)} tiles / carton</p>
                )}
                <p className="mt-1 text-sm">
                  {currency(p.price)}{" "}
                  <span className="text-xs text-muted-foreground">
                    · {p.stock_qty} {p.unit} in stock
                  </span>
                </p>
              </button>
            ))}
            {results.length === 0 && (
              <p className="text-sm text-muted-foreground">No products match that search.</p>
            )}
          </div>
        </div>

        <div className="rounded-lg border border-border bg-card p-4">
          <h2 className="font-semibold">Current sale</h2>
          <div className="mt-3 divide-y divide-border">
            {lines.length === 0 && (
              <p className="py-6 text-sm text-muted-foreground">
                Tap a product to start the invoice.
              </p>
            )}
            {lines.map((l) => {
              const box = perCarton(l.product);
              const setQty = (qty: number) =>
                setLines((prev) =>
                  prev.map((x) =>
                    x.product.id === l.product.id
                      ? { ...x, qty: Math.max(box ? 0 : 1, qty) }
                      : x,
                  ),
                );
              const cartons = box ? Math.floor(l.qty / box) : 0;
              const loose = box ? l.qty % box : 0;
              return (
                <div key={l.product.id} className="py-3">
                  <div className="flex items-center gap-3">
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium">{l.product.name}</p>
                      <p className="text-xs text-muted-foreground">
                        {currency(l.product.price)} / {l.product.unit}
                        {box ? ` · ${box} per carton` : ""}
                      </p>
                    </div>
                    {!box && (
                      <NumberInput
                        min={1}
                        className="w-20"
                        value={l.qty}
                        onChange={(e) => setQty(Math.max(1, Math.floor(Number(e.target.value) || 1)))}
                      />
                    )}
                    <span className="w-24 text-right text-sm">
                      {currency(Number(l.product.price) * l.qty)}
                    </span>
                    <button
                      onClick={() =>
                        setLines((prev) => prev.filter((x) => x.product.id !== l.product.id))
                      }
                      aria-label={`Remove ${l.product.name}`}
                      className="text-muted-foreground hover:text-destructive"
                    >
                      <Trash2 className="size-4" />
                    </button>
                  </div>
                  {box > 0 && (
                    <div className="mt-2 grid grid-cols-2 gap-2">
                      <div className="space-y-1">
                        <Label className="text-xs">Cartons</Label>
                        <NumberInput
                          value={cartons}
                          onChange={(e) =>
                            setQty(Math.max(0, Number(e.target.value) || 0) * box + loose)
                          }
                        />
                      </div>
                      <div className="space-y-1">
                        <Label className="text-xs">Extra tiles</Label>
                        <NumberInput
                          value={loose}
                          onChange={(e) =>
                            setQty(cartons * box + Math.max(0, Number(e.target.value) || 0))
                          }
                        />
                      </div>
                      <p className="col-span-2 text-xs text-muted-foreground">
                        {l.qty} {l.product.unit} total
                      </p>
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          <div className="mt-4 space-y-3 border-t border-border pt-4">
            <div className="space-y-2">
              <Label>Customer</Label>
              <select
                className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                value={customerId}
                onChange={(e) => setCustomerId(e.target.value)}
              >
                <option value="">Walk-in / cash customer</option>
                {customers.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
              {!customerId && (
                <div className="space-y-2 rounded-md border border-border p-3">
                  <Input
                    value={walkIn}
                    onChange={(e) => setWalkIn(e.target.value)}
                    placeholder="Customer name on invoice"
                    maxLength={100}
                  />
                  <Input
                    value={walkInPhone}
                    onChange={(e) => setWalkInPhone(e.target.value)}
                    placeholder="Phone number (optional)"
                    maxLength={30}
                  />
                  <label className="flex items-center gap-2 text-xs text-muted-foreground">
                    <input
                      type="checkbox"
                      className="size-4 accent-brass"
                      checked={saveWalkIn}
                      onChange={(e) => setSaveWalkIn(e.target.checked)}
                    />
                    Save this customer to Customers records
                  </label>
                </div>
              )}
            </div>

            <div className="space-y-2">
              <Label>Delivery / receiving date (optional)</Label>
              <Input
                type="date"
                value={deliveryDate}
                onChange={(e) => setDeliveryDate(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Use this for marble or tile orders that will be delivered later.
              </p>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-2">
                <Label>Discount (PKR)</Label>
                <NumberInput
                  value={discount}
                  onChange={(e) => setDiscount(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label>Amount paid</Label>
                <NumberInput
                  value={amountPaid}
                  onChange={(e) => setAmountPaid(e.target.value)}
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Payment method</Label>
              <div className="grid grid-cols-3 gap-2">
                {(["cash", "bank", "credit"] as const).map((m) => (
                  <button
                    key={m}
                    onClick={() => setMethod(m)}
                    className={`rounded-md border px-3 py-2 text-sm capitalize ${
                      method === m
                        ? "border-foreground bg-foreground text-background"
                        : "border-border"
                    }`}
                  >
                    {m}
                  </button>
                ))}
              </div>
            </div>

            <div className="space-y-2">
              <Label>Notes</Label>
              <Input value={notes} onChange={(e) => setNotes(e.target.value)} maxLength={200} />
            </div>

            <div className="space-y-1 border-t border-border pt-3 text-sm">
              <div className="flex justify-between text-muted-foreground">
                <span>Subtotal</span>
                <span>{currency(subtotal)}</span>
              </div>
              <div className="flex justify-between text-muted-foreground">
                <span>Discount</span>
                <span>-{currency(Number(discount) || 0)}</span>
              </div>
              <div className="flex justify-between text-lg font-semibold">
                <span>Total</span>
                <span>{currency(total)}</span>
              </div>
              {balanceDue > 0 && (
                <div className="flex justify-between text-destructive font-medium">
                  <span>Balance due</span>
                  <span>{currency(balanceDue)}</span>
                </div>
              )}
            </div>

            <Button
              variant="brass"
              size="xl"
              className="w-full"
              disabled={checkout.isPending}
              onClick={() => checkout.mutate()}
            >
              {checkout.isPending ? "Saving…" : "Complete sale"}
            </Button>
          </div>
        </div>
      </div>
    </AdminShell>
  );
}