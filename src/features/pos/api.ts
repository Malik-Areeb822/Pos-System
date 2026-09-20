import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api-client";

export type Product = {
  id: string;
  name: string;
  sku: string | null;
  category: "sanitary" | "hardware";
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

export type Customer = {
  id: string;
  name: string;
  phone: string | null;
  email: string | null;
  address: string | null;
  outstanding_balance: number;
  created_at: string;
};

export function useProductsForPOS(category?: string) {
  return useQuery({
    queryKey: ["pos-products", category],
    queryFn: () => api.products.list(category),
  });
}

export function useCustomersForPOS() {
  return useQuery({
    queryKey: ["pos-customers"],
    queryFn: () => api.customers.list(),
  });
}
