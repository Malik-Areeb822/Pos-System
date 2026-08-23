import { Link } from "@tanstack/react-router";

import { categoryMeta, currency, productImage, type Product } from "@/lib/catalog";

export function ProductCard({ product }: { product: Product }) {
  return (
    <Link
      to="/website/product/$productId"
      params={{ productId: product.id }}
      className="group flex flex-col overflow-hidden border border-border bg-card transition-shadow hover:shadow-lift"
    >
      <div className="aspect-4/3 overflow-hidden bg-limestone">
        <img
          src={productImage(product)}
          alt={product.name}
          loading="lazy"
          className="size-full object-cover transition-transform duration-700 group-hover:scale-105"
        />
      </div>
      <div className="flex flex-1 flex-col p-5">
        <p className="eyebrow text-brass">{categoryMeta(product.category).label}</p>
        <h3 className="mt-2 text-xl">{product.name}</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {[product.color, product.size, product.finish].filter(Boolean).join(" · ")}
        </p>
        <div className="mt-auto flex items-end justify-between pt-5">
          <span className="text-sm">
            <span className="font-display text-2xl">{currency(product.price)}</span>
            <span className="text-muted-foreground"> / {product.unit}</span>
          </span>
          <span className="eyebrow text-foreground/70 group-hover:text-brass">Details →</span>
        </div>
      </div>
    </Link>
  );
}
