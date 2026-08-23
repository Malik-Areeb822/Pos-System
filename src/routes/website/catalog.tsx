import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { SiteShell, PageHeader } from "@/components/website/SiteShell";
import { ProductCard } from "@/components/website/ProductCard";
import { CATEGORIES, fetchProducts, type Category } from "@/lib/catalog";
import { Skeleton } from "@/components/ui/skeleton";

type CatalogSearch = { category?: Category; color?: string; finish?: string };

const CATEGORY_KEYS = CATEGORIES.map((c) => c.key);

export const Route = createFileRoute("/website/catalog")({
  validateSearch: (search: Record<string, unknown>): CatalogSearch => ({
    category: CATEGORY_KEYS.includes(search.category as Category)
      ? (search.category as Category)
      : undefined,
    color: typeof search.color === "string" ? search.color : undefined,
    finish: typeof search.finish === "string" ? search.finish : undefined,
  }),
  head: () => ({
    meta: [
      { title: "Product Catalog — City Tiles" },
      {
        name: "description",
        content:
          "Browse marble slabs, porcelain and ceramic tiles, terrazzo flooring chips and sanitary ware with sizes, finishes and factory prices.",
      },
      { property: "og:title", content: "Product Catalog — City Tiles" },
      {
        property: "og:description",
        content: "Marble, tiles, flooring chips and sanitary ware with factory-direct pricing.",
      },
    ],
  }),
  component: CatalogPage,
});

function FilterRow({
  label,
  options,
  value,
  onSelect,
}: {
  label: string;
  options: string[];
  value?: string;
  onSelect: (v?: string) => void;
}) {
  if (options.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="eyebrow mr-2 text-muted-foreground">{label}</span>
      <button
        onClick={() => onSelect(undefined)}
        className={`border px-3 py-1.5 text-xs tracking-wide transition-colors ${
          !value ? "border-foreground bg-foreground text-background" : "border-border hover:border-foreground"
        }`}
      >
        All
      </button>
      {options.map((opt) => (
        <button
          key={opt}
          onClick={() => onSelect(opt)}
          className={`border px-3 py-1.5 text-xs tracking-wide transition-colors ${
            value === opt
              ? "border-foreground bg-foreground text-background"
              : "border-border hover:border-foreground"
          }`}
        >
          {opt}
        </button>
      ))}
    </div>
  );
}

function CatalogPage() {
  const search = Route.useSearch();
  const navigate = useNavigate({ from: Route.fullPath });

  const { data, isLoading } = useQuery({
    queryKey: ["products"],
    queryFn: () => fetchProducts(),
  });

  const products = data ?? [];
  const byCategory = useMemo(
    () => (search.category ? products.filter((p) => p.category === search.category) : products),
    [products, search.category],
  );

  const colors = useMemo(
    () => [...new Set(byCategory.map((p) => p.color).filter(Boolean) as string[])].sort(),
    [byCategory],
  );
  const finishes = useMemo(
    () => [...new Set(byCategory.map((p) => p.finish).filter(Boolean) as string[])].sort(),
    [byCategory],
  );

  const filtered = byCategory.filter(
    (p) =>
      (!search.color || p.color === search.color) && (!search.finish || p.finish === search.finish),
  );

  const update = (patch: Partial<CatalogSearch>) =>
    navigate({ search: (prev: CatalogSearch) => ({ ...prev, ...patch }) });

  return (
    <SiteShell>
      <PageHeader
        eyebrow="Catalog"
        title="Everything we cut, stock and supply"
        intro="Prices shown are factory rates for retail quantities. Bulk and trade rates are quoted per project."
      />

      <div className="mx-auto max-w-7xl px-5 py-12">
        <div className="flex flex-wrap gap-2 border-b border-border pb-6">
          <button
            onClick={() => update({ category: undefined, color: undefined, finish: undefined })}
            className={`px-4 py-2 text-sm tracking-wide ${
              !search.category ? "bg-foreground text-background" : "hover:text-brass"
            }`}
          >
            All products
          </button>
          {CATEGORIES.map((cat) => (
            <button
              key={cat.key}
              onClick={() => update({ category: cat.key, color: undefined, finish: undefined })}
              className={`px-4 py-2 text-sm tracking-wide ${
                search.category === cat.key ? "bg-foreground text-background" : "hover:text-brass"
              }`}
            >
              {cat.label}
            </button>
          ))}
        </div>

        <div className="mt-6 space-y-3">
          <FilterRow
            label="Colour"
            options={colors}
            value={search.color}
            onSelect={(v) => update({ color: v })}
          />
          <FilterRow
            label="Size / Finish"
            options={finishes}
            value={search.finish}
            onSelect={(v) => update({ finish: v })}
          />
        </div>

        {isLoading ? (
          <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-96 w-full" />
            ))}
          </div>
        ) : filtered.length === 0 ? (
          <p className="mt-16 text-muted-foreground">
            No products match these filters. Try clearing a filter or call us for special orders.
          </p>
        ) : (
          <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {filtered.map((product) => (
              <ProductCard key={product.id} product={product} />
            ))}
          </div>
        )}
      </div>
    </SiteShell>
  );
}
