import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useRef, useState } from "react";
import { Search, Upload, Download, Trash2 } from "lucide-react";
import { toast } from "sonner";
import * as XLSX from "xlsx";

import { AdminShell } from "@/components/admin/AdminShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { useRoles } from "@/features/auth/api";
import { CATEGORIES, currency, type Category, type Product, useProducts, useCreateProduct, useUpdateProduct, useDeleteProduct, useImportProducts, useExportProducts } from "@/features/inventory/api";

export const Route = createFileRoute("/_authenticated/admin/inventory")({
  component: InventoryPage,
});

type Draft = {
  name: string;
  sku: string;
  category: Category;
  description: string;
  color: string;
  size: string;
  unit: string;
  price: string;
  stock_qty: string;
  pieces_per_carton: string;
  low_stock_threshold: string;
};

const emptyDraft: Draft = {
  name: "",
  sku: "",
  category: "marble",
  description: "",
  color: "",
  size: "",
  unit: "sq ft",
  price: "0",
  stock_qty: "0",
  pieces_per_carton: "8",
  low_stock_threshold: "10",
};

function toDraft(p: Product): Draft {
  return {
    name: p.name,
    sku: p.sku ?? "",
    category: p.category,
    description: p.description ?? "",
    color: p.color ?? "",
    size: p.size ?? "",
    unit: p.unit,
    price: String(p.price),
    stock_qty: String(p.stock_qty),
    pieces_per_carton: p.pieces_per_carton ? String(p.pieces_per_carton) : "",
    low_stock_threshold: String(p.low_stock_threshold),
  };
}

const EXPORT_COLUMNS = [
  "Stock code",
  "Name",
  "Category",
  "Colour",
  "Size",
  "Unit",
  "Price",
  "Stock qty",
  "Tiles per carton",
  "Low stock alert",
  "Description",
] as const;

function InventoryPage() {
  const queryClient = useQueryClient();
  const { data: products = [], isLoading } = useProducts();
  const { data: roles = [] } = useRoles();
  const isAdmin = roles.includes("admin");

  const createProduct = useCreateProduct();
  const updateProduct = useUpdateProduct();
  const deleteProduct = useDeleteProduct();
  const importProducts = useImportProducts();
  const exportProducts = useExportProducts();

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<Product | null>(null);
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [filter, setFilter] = useState<Category | "all">("all");
  const [search, setSearch] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);

  const save = useMutation({
    mutationFn: async () => {
      const payload = {
        name: draft.name.trim(),
        sku: draft.sku.trim() || null,
        category: draft.category,
        description: draft.description.trim(),
        color: draft.color.trim() || null,
        size: draft.size.trim() || null,
        unit: draft.unit.trim() || "unit",
        price: Number(draft.price) || 0,
        stock_qty: Number(draft.stock_qty) || 0,
        pieces_per_carton:
          draft.category === "tiles" && Number(draft.pieces_per_carton) > 0
            ? Number(draft.pieces_per_carton)
            : null,
        low_stock_threshold: Number(draft.low_stock_threshold) || 0,
      };
      if (!payload.name) throw new Error("Product name is required");
      if (editing) {
        await updateProduct.mutateAsync({ id: editing.id, input: payload });
      } else {
        await createProduct.mutateAsync(payload);
      }
    },
    onSuccess: () => {
      toast.success(editing ? "Product updated" : "Product added");
      queryClient.invalidateQueries({ queryKey: ["products"] });
      setOpen(false);
      setEditing(null);
      setDraft(emptyDraft);
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const remove = useMutation({
    mutationFn: async (product: Product) => {
      await deleteProduct.mutateAsync(product.id);
    },
    onSuccess: () => {
      toast.success("Product deleted");
      queryClient.invalidateQueries({ queryKey: ["products"] });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  const importRows = useMutation({
    mutationFn: async (file: File) => {
      const book = XLSX.read(await file.arrayBuffer(), { type: "array" });
      const sheet = book.Sheets[book.SheetNames[0]];
      const rows = XLSX.utils.sheet_to_json<Record<string, unknown>>(sheet, { defval: "" });
      const pick = (row: Record<string, unknown>, ...keys: string[]) => {
        for (const key of keys) {
          const found = Object.keys(row).find(
            (k) => k.trim().toLowerCase() === key.toLowerCase(),
          );
          if (found && String(row[found]).trim() !== "") return String(row[found]).trim();
        }
        return "";
      };
      const valid = new Set(CATEGORIES.map((c) => c.key));
      let created = 0;
      let updated = 0;

      for (const row of rows) {
        const sku = pick(row, "Stock code", "sku", "code");
        const name = pick(row, "Name", "product") || sku;
        if (!name) continue;
        const rawCategory = pick(row, "Category").toLowerCase();
        const category = (valid.has(rawCategory as Category) ? rawCategory : "tiles") as Category;
        const perCarton = Number(pick(row, "Tiles per carton", "pieces_per_carton")) || 0;
        const payload = {
          name,
          sku: sku || null,
          category,
          description: pick(row, "Description"),
          color: pick(row, "Colour", "Color") || null,
          size: pick(row, "Size") || null,
          unit: pick(row, "Unit") || (category === "tiles" ? "tile" : "unit"),
          price: Number(pick(row, "Price")) || 0,
          stock_qty: Number(pick(row, "Stock qty", "Stock")) || 0,
          pieces_per_carton: category === "tiles" && perCarton > 0 ? perCarton : null,
          low_stock_threshold: Number(pick(row, "Low stock alert", "low_stock_threshold")) || 10,
        };
        const existing = sku
          ? products.find((p) => (p.sku ?? "").toLowerCase() === sku.toLowerCase())
          : products.find((p) => p.name.toLowerCase() === name.toLowerCase());
        if (existing) {
          await updateProduct.mutateAsync({ id: existing.id, input: payload });
          updated += 1;
        } else {
          await createProduct.mutateAsync(payload);
          created += 1;
        }
      }
      return { created, updated };
    },
    onSuccess: ({ created, updated }) => {
      toast.success(`Import complete — ${created} added, ${updated} updated`);
      queryClient.invalidateQueries({ queryKey: ["products"] });
    },
    onError: (err: Error) => toast.error(err.message),
  });

  function exportExcel() {
    exportProducts.mutate();
  }

  const rows = useMemo(() => {
    const q = search.trim().toLowerCase();
    return products
      .filter((p) => filter === "all" || p.category === filter)
      .filter((p) =>
        !q
          ? true
          : [p.name, p.sku, p.color, p.size, p.unit, p.description]
              .filter(Boolean)
              .some((v) => String(v).toLowerCase().includes(q)),
      );
  }, [products, filter, search]);

  const field = (key: keyof Draft) => ({
    value: draft[key],
    onChange: (e: { target: { value: string } }) =>
      setDraft((d) => ({ ...d, [key]: e.target.value })),
  });

  return (
    <AdminShell
      title="Inventory"
      actions={
        isAdmin ? (
          <>
            <input
              ref={fileRef}
              type="file"
              accept=".xlsx,.xls,.csv"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) importRows.mutate(file);
                e.target.value = "";
              }}
            />
            <Button size="sm" variant="outline" onClick={exportExcel}>
              <Download className="size-4" /> Export Excel
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={importRows.isPending}
              onClick={() => fileRef.current?.click()}
            >
              <Upload className="size-4" /> {importRows.isPending ? "Importing…" : "Import Excel"}
            </Button>
            <Dialog
              open={open}
              onOpenChange={(v) => {
                setOpen(v);
                if (!v) {
                  setEditing(null);
                  setDraft(emptyDraft);
                }
              }}
            >
              <DialogTrigger asChild>
                <Button size="sm" variant="brass">
                  Add product
                </Button>
              </DialogTrigger>
              <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
                <DialogHeader>
                  <DialogTitle>{editing ? "Edit product" : "Add product"}</DialogTitle>
                </DialogHeader>
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2 sm:col-span-2">
                    <Label>Name</Label>
                    <Input {...field("name")} maxLength={120} />
                  </div>
                  <div className="space-y-2">
                    <Label>Category</Label>
                    <select
                      className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                      value={draft.category}
                      onChange={(e) =>
                        setDraft((d) => ({ ...d, category: e.target.value as Category }))
                      }
                    >
                      {CATEGORIES.map((c) => (
                        <option key={c.key} value={c.key}>
                          {c.label}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div className="space-y-2">
                    <Label>Stock code (e.g. 100B)</Label>
                    <Input {...field("sku")} maxLength={40} placeholder="100B" />
                  </div>
                  <div className="space-y-2">
                    <Label>Colour</Label>
                    <Input {...field("color")} maxLength={60} />
                  </div>
<div className="space-y-2">
                      <Label>Size</Label>
                      <Input {...field("size")} maxLength={60} />
                    </div>
                  <div className="space-y-2">
                    <Label>Unit</Label>
                    <Input {...field("unit")} maxLength={20} />
                  </div>
                  <div className="space-y-2">
                    <Label>Price (PKR)</Label>
                    <Input type="number" min="0" {...field("price")} />
                  </div>
                  <div className="space-y-2">
                    <Label>Stock quantity</Label>
                    <Input type="number" min="0" {...field("stock_qty")} />
                  </div>
                  <div className="space-y-2">
                    <Label>Low stock alert at</Label>
                    <Input type="number" min="0" {...field("low_stock_threshold")} />
                  </div>
                  {draft.category === "tiles" && (
                    <div className="space-y-2">
                      <Label>Tiles per carton</Label>
                      <Input type="number" min="1" {...field("pieces_per_carton")} />
                      <p className="text-xs text-muted-foreground">
                        Used at the counter to bill by carton (e.g. 8 tiles per carton).
                      </p>
                    </div>
                  )}
                  <div className="space-y-2 sm:col-span-2">
                    <Label>Description</Label>
                    <Textarea rows={3} {...field("description")} maxLength={600} />
                  </div>
                </div>
                <Button
                  variant="brass"
                  className="mt-2 w-full"
                  disabled={save.isPending}
                  onClick={() => save.mutate()}
                >
                  {save.isPending ? "Saving…" : "Save product"}
                </Button>
              </DialogContent>
            </Dialog>
          </>
        ) : null
      }
    >
      <div className="relative mb-4">
        <Search className="absolute left-3 top-2.5 size-4 text-muted-foreground" />
        <Input
          className="pl-9"
          placeholder="Search by stock code, name, colour or size"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      <div className="flex flex-wrap gap-2">
        {(["all", ...CATEGORIES.map((c) => c.key)] as (Category | "all")[]).map((key) => (
          <button
            key={key}
            onClick={() => setFilter(key)}
            className={`rounded-md border px-3 py-1.5 text-sm ${
              filter === key ? "border-foreground bg-foreground text-background" : "border-border"
            }`}
          >
            {key === "all" ? "All" : CATEGORIES.find((c) => c.key === key)!.label}
          </button>
        ))}
      </div>

      <div className="mt-5 overflow-x-auto rounded-lg border border-border bg-card">
        <table className="w-full min-w-[720px] text-sm">
          <thead className="border-b border-border text-left text-xs uppercase tracking-wider text-muted-foreground">
            <tr>
              <th className="px-4 py-3">Product</th>
              <th className="px-4 py-3">Variant</th>
              <th className="px-4 py-3 text-right">Price</th>
              <th className="px-4 py-3 text-right">Stock</th>
              <th className="px-4 py-3" />
            </tr>
          </thead>
          <tbody>
            {isLoading && (
              <tr>
                <td colSpan={5} className="px-4 py-6 text-muted-foreground">
                  Loading inventory…
                </td>
              </tr>
            )}
            {!isLoading && rows.length === 0 && (
              <tr>
                <td colSpan={5} className="px-4 py-6 text-muted-foreground">
                  No products match that search.
                </td>
              </tr>
            )}
            {rows.map((p) => (
              <tr key={p.id} className="border-b border-border last:border-0">
                <td className="px-4 py-3">
                  <p className="font-medium">{p.name}</p>
                  <p className="text-xs text-muted-foreground">{p.sku ?? "—"}</p>
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {[p.color, p.size].filter(Boolean).join(" · ") || "—"}
                </td>
                <td className="px-4 py-3 text-right">
                  {currency(p.price)}{" "}
                  <span className="text-xs text-muted-foreground">/ {p.unit}</span>
                </td>
                <td
                  className={`px-4 py-3 text-right ${
                    p.stock_qty <= p.low_stock_threshold ? "text-destructive" : ""
                  }`}
                >
                  {p.stock_qty}
                </td>
                <td className="px-4 py-3">
                  {isAdmin && (
                    <div className="flex justify-end gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setEditing(p);
                          setDraft(toDraft(p));
                          setOpen(true);
                        }}
                      >
                        Edit
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={remove.isPending}
                        onClick={() => {
                          if (confirm(`Delete "${p.name}" from inventory?`)) remove.mutate(p);
                        }}
                      >
                        <Trash2 className="size-4 text-destructive" />
                      </Button>
                    </div>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {!isAdmin && (
        <p className="mt-3 text-xs text-muted-foreground">
          You have view-only access to inventory. Only the admin can add, edit or delete products.
        </p>
      )}
    </AdminShell>
  );
}