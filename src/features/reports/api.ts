import { api } from "@/lib/api-client";
import { useQuery } from "@tanstack/react-query";

export type DashboardData = {
  total_revenue: number;
  total_orders: number;
  total_customers: number;
  outstanding_balance: number;
  low_stock_count: number;
  recent_invoices: Invoice[];
};

export type Invoice = {
  id: string;
  invoice_no: string;
  customer_id: string | null;
  customer_name: string;
  subtotal: number;
  discount: number;
  total: number;
  amount_paid: number;
  payment_method: "cash" | "bank" | "credit";
  notes: string | null;
  delivery_date: string | null;
  created_at: string;
};

export type ReportParams = {
  from?: string;
  to?: string;
  category?: string;
};

export type SalesReport = {
  total_revenue: number;
  total_orders: number;
  average_order_value: number;
  by_payment_method: Record<string, number>;
  by_category: Record<string, number>;
  daily: Array<{ date: string; revenue: number; orders: number }>;
};

export type Product = {
  id: string;
  name: string;
  sku: string | null;
  category: "marble" | "tiles" | "chips" | "sanitary";
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

export type InventoryReport = {
  total_products: number;
  total_stock_value: number;
  low_stock_products: Product[];
  out_of_stock_products: Product[];
  by_category: Record<string, { count: number; value: number }>;
};

export function useDashboard() {
  return useQuery({
    queryKey: ["dashboard"],
    queryFn: () => api.reports.getDashboard(),
  });
}

export function useSalesReport(params: ReportParams) {
  return useQuery({
    queryKey: ["sales-report", params],
    queryFn: () => api.reports.getSales(params),
  });
}

export function useInventoryReport() {
  return useQuery({
    queryKey: ["inventory-report"],
    queryFn: () => api.reports.getInventory(),
  });
}