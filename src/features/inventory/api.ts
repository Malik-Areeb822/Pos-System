import { api } from "@/lib/api-client";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

export type Category = "sanitary";

export type Product = {
  id: string;
  name: string;
  sku: string | null;
  category: Category;
  description: string;
  color: string | null;
  size: string | null;
  finish: string | null;
  company: string | null;
  unit: string;
  price: number;
  pieces_per_carton: number | null;
  area_per_tile: number | null;
  stock_qty: number;
  low_stock_threshold: number;
  image_url: string | null;
  is_published: boolean;
};

export type CreateProductInput = {
  name: string;
  sku: string;
  category: "sanitary";
  description: string;
  color: string;
  size: string;
  finish: string;
  company?: string;
  unit: string;
  price: number;
  pieces_per_carton?: number;
  area_per_tile?: number;
  stock_qty: number;
  low_stock_threshold: number;
  image_url: string;
  is_published: boolean;
};

export type ImportResult = {
  imported: number;
  errors: string[];
};

export function useProducts(category?: string) {
  return useQuery({
    queryKey: ["products", category],
    queryFn: () => api.products.list(category),
  });
}

export function useProduct(id: string) {
  return useQuery({
    queryKey: ["product", id],
    queryFn: () => api.products.get(id),
    enabled: !!id,
  });
}

export function useCreateProduct() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateProductInput) => api.products.create(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["products"] });
    },
  });
}

export function useUpdateProduct() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: Partial<CreateProductInput> }) =>
      api.products.update(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["products"] });
    },
  });
}

export function useDeleteProduct() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.products.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["products"] });
    },
  });
}

export function useImportProducts() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (file: File) => api.products.import(await file.text()),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["products"] });
    },
  });
}

export const CATEGORIES = [
  {
    key: "sanitary" as const,
    label: "Sanitary",
    blurb: "Pipes, fittings, commodes, basins, mixers and accessories.",
    image: "/images/cat-sanitary.jpg",
  },
] as const;

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
