const sanitaryImg = "/images/cat-sanitary.jpg";
const hardwareImg = "/images/cat-sanitary.jpg";

export type Category = "sanitary" | "hardware";

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
  area_per_tile: number | null;
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
    key: "sanitary",
    label: "Sanitary",
    blurb: "Pipes, fittings, commodes, basins, mixers and accessories.",
    image: sanitaryImg,
  },
  {
    key: "hardware",
    label: "Hardware",
    blurb: "Tools, fasteners, valves, and general hardware supplies.",
    image: hardwareImg,
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
