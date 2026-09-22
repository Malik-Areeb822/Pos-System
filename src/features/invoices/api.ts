import { api } from "@/lib/api-client";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

export type PaymentMethod = "cash" | "bank" | "credit";

export type Invoice = {
  id: string;
  invoice_no: string;
  customer_id: string | null;
  customer_name: string;
  subtotal: number;
  discount: number;
  total: number;
  previous_balance: number;
  amount_paid: number;
  payment_method: PaymentMethod;
  notes: string | null;
  delivery_date: string | null;
  created_at: string;
  carried_to_invoice_id: string | null;
};

export type InvoiceItem = {
  id: string;
  invoice_id: string;
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
  purchase_price: number;
  total_area: number | null;
};

export type CreateInvoiceInput = {
  customer_id: string | null;
  customer_name: string;
  items: CreateInvoiceItemInput[];
  subtotal: number;
  discount?: number;
  total: number;
  amount_paid?: number;
  payment_method: PaymentMethod;
  notes?: string;
  delivery_date?: string;
};

export type CreateInvoiceItemInput = {
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
  purchase_price: number;
  total_area?: number | null;
};

export function useInvoices() {
  return useQuery({
    queryKey: ["invoices"],
    queryFn: () => api.invoices.list(),
  });
}

export function useInvoiceSearch(query: string) {
  const trimmed = query.trim();
  return useQuery({
    queryKey: ["invoices", "search", trimmed],
    queryFn: () => api.invoices.list({ query: trimmed }),
    enabled: trimmed.length > 0,
  });
}

export function useInvoice(id: string) {
  return useQuery({
    queryKey: ["invoice", id],
    queryFn: () => api.invoices.getWithItems(id),
    enabled: !!id,
  });
}

export function useCreateInvoice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateInvoiceInput) => api.invoices.create(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["invoices"] });
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export type MarkInvoicePaidInput = {
  id: string;
  amount: number;
  payment_method: PaymentMethod;
};

export function useMarkInvoicePaid() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: MarkInvoicePaidInput) => api.invoices.markPaid(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["invoices"] });
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function usePrintReceipt() {
  return useMutation({
    mutationFn: (id: string) => api.invoices.printReceipt(id),
  });
}

export function usePrintInvoicePdf() {
  return useMutation({
    mutationFn: (id: string) => api.invoices.printInvoicePdf(id),
  });
}

export const formatDate = (iso: string) =>
  new Date(iso).toLocaleDateString("en-GB", { day: "2-digit", month: "short", year: "numeric" });
