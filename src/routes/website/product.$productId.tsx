import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";

import { SiteShell } from "@/components/website/SiteShell";
import { ProductCard } from "@/components/website/ProductCard";
import { InquiryForm } from "@/components/website/InquiryForm";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { categoryMeta, currency, fetchProduct, fetchProducts, productImage } from "@/lib/catalog";

export const Route = createFileRoute("/website/product/$productId")({
  head: () => ({
    meta: [
      { title: "Product Details — City Tiles" },
      {
        name: "description",
        content:
          "Specifications, available colours and finishes, and factory pricing for this product from City Tiles.",
      },
      { property: "og:title", content: "Product Details — City Tiles" },
      {
        property: "og:description",
        content: "Specifications, finishes and factory pricing.",
      },
    ],
  }),
  component: ProductPage,
});

function ProductPage() {
  const { productId } = Route.useParams();

  const { data: product, isLoading } = useQuery({
    queryKey: ["product", productId],
    queryFn: () => fetchProduct(productId),
  });
  const { data: all } = useQuery({ queryKey: ["products"], queryFn: () => fetchProducts() });

  if (isLoading) {
    return (
      <SiteShell>
        <div className="mx-auto max-w-7xl px-5 py-20">
          <Skeleton className="h-[28rem] w-full" />
        </div>
      </SiteShell>
    );
  }

  if (!product) {
    return (
      <SiteShell>
        <div className="mx-auto max-w-3xl px-5 py-32 text-center">
          <h1 className="text-4xl">Product not available</h1>
          <p className="mt-4 text-muted-foreground">
            This item may have been removed from the catalog.
          </p>
          <Button asChild variant="brass" size="xl" className="mt-8">
            <Link to="/website/catalog">Back to catalog</Link>
          </Button>
        </div>
      </SiteShell>
    );
  }

  const meta = categoryMeta(product.category);
  const related = (all ?? [])
    .filter((p) => p.category === product.category && p.id !== product.id)
    .slice(0, 3);

  const specs = [
    ["Category", meta.label],
    ["Colour", product.color ?? "—"],
    ["Size", product.size ?? "—"],
    ["Finish", product.finish ?? "—"],
    ["Sold by", product.unit],
    ["Reference", product.sku ?? "—"],
  ];

  return (
    <SiteShell>
      <div className="mx-auto max-w-7xl px-5 py-10">
        <Link to="/website/catalog" className="eyebrow text-muted-foreground hover:text-brass">
          ← Back to catalog
        </Link>
      </div>

      <section className="mx-auto grid max-w-7xl gap-14 px-5 pb-20 lg:grid-cols-2">
        <img
          src={productImage(product)}
          alt={product.name}
          className="aspect-4/5 w-full object-cover shadow-stone"
        />
        <div>
          <p className="eyebrow text-brass">{meta.label}</p>
          <h1 className="mt-3 text-4xl md:text-5xl">{product.name}</h1>
          <p className="mt-5 text-muted-foreground">{product.description}</p>
          <p className="mt-8 font-display text-4xl">
            {currency(product.price)}
            <span className="ml-1 font-sans text-base text-muted-foreground">
              / {product.unit}
            </span>
          </p>

          <dl className="mt-10 grid grid-cols-2 gap-x-8 gap-y-5 border-t border-border pt-8">
            {specs.map(([label, value]) => (
              <div key={label}>
                <dt className="eyebrow text-muted-foreground">{label}</dt>
                <dd className="mt-1">{value}</dd>
              </div>
            ))}
          </dl>

          <div className="mt-10 border border-border bg-card p-6 md:p-8">
            <h2 className="text-2xl">Request a quote for {product.name}</h2>
            <div className="mt-6">
              <InquiryForm
                inquiryType="quote"
                submitLabel="Request Quote"
                presetMessage={`I'd like a quote for ${product.name}${product.size ? ` (${product.size})` : ""}. Quantity: `}
              />
            </div>
          </div>
        </div>
      </section>

      {related.length > 0 && (
        <section className="stone-surface border-t border-border">
          <div className="mx-auto max-w-7xl px-5 py-20">
            <h2 className="text-3xl md:text-4xl">More {meta.label.toLowerCase()}</h2>
            <div className="mt-10 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
              {related.map((p) => (
                <ProductCard key={p.id} product={p} />
              ))}
            </div>
          </div>
        </section>
      )}
    </SiteShell>
  );
}
