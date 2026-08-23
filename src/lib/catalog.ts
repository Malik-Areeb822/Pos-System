import { supabase } from "@/integrations/supabase/client";
const marbleImg = "/images/cat-marble.jpg";
const tilesImg = "/images/cat-tiles.jpg";
const chipsImg = "/images/cat-chips.jpg";
const sanitaryImg = "/images/cat-sanitary.jpg";

export type Category = "marble" | "tiles" | "chips" | "sanitary";

export type Product = {
  id: string;
  name: string;
  sku: string | null;
  category: Category;
  description: string;
  color: string | null;
  size: string | null;
  finish: string | null;
  unit: string;
  price: number;
  pieces_per_carton: number | null;
  stock_qty: number;
  low_stock_threshold: number;
  image_url: string | null;
  is_published: boolean;
};

export const CATEGORIES: {
  key: Category;
  label: string;
  blurb: string;
  image: string;
}[] = [
  {
    key: "marble",
    label: "Marble",
    blurb: "Slabs and tiles cut from natural stone for floors, walls and counters.",
    image: marbleImg,
  },
  {
    key: "tiles",
    label: "Tiles",
    blurb: "Porcelain and ceramic in every size, finish and pattern we stock.",
    image: tilesImg,
  },
  {
    key: "chips",
    label: "Flooring Chips",
    blurb: "Graded marble chips by colour for terrazzo-style cast flooring.",
    image: chipsImg,
  },
  {
    key: "sanitary",
    label: "Sanitary",
    blurb: "Commodes, basins, mixers and fittings for homes and TB lounges.",
    image: sanitaryImg,
  },
];

export function categoryMeta(key: Category) {
  return CATEGORIES.find((c) => c.key === key)!;
}

export function productImage(product: Pick<Product, "image_url" | "category">) {
  return product.image_url || categoryMeta(product.category).image;
}

export const currency = (value: number) =>
  new Intl.NumberFormat("en-PK", {
    style: "currency",
    currency: "PKR",
    maximumFractionDigits: 0,
  }).format(Number(value || 0));

export async function fetchProducts(category?: Category) {
  let query = supabase.from("products").select("*").order("category").order("name");
  if (category) query = query.eq("category", category);
  const { data, error } = await query;
  if (error) throw error;
  return (data ?? []) as unknown as Product[];
}

export async function fetchProduct(id: string) {
  const { data, error } = await supabase.from("products").select("*").eq("id", id).maybeSingle();
  if (error) throw error;
  return (data as unknown as Product) ?? null;
}
