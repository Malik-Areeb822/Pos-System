import { useAuthStore } from "@/lib/auth-store";

export type Role = "admin" | "cashier";
export type AccountStatus = "pending" | "approved" | "rejected" | "suspended";

export type StaffProfile = {
  id: string;
  full_name: string;
  email: string | null;
  phone: string | null;
  employee_id: string | null;
  status: AccountStatus;
  created_at: string;
  approved_at: string | null;
  rejected_at: string | null;
};
export type PaymentMethod = "cash" | "bank" | "credit";

export type Customer = {
  id: string;
  name: string;
  phone: string | null;
  email: string | null;
  address: string | null;
  outstanding_balance: number;
  created_at: string;
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
  payment_method: PaymentMethod;
  notes: string | null;
  delivery_date: string | null;
  created_at: string;
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
};

export function useSession() {
  const user = useAuthStore((s) => s.user);
  return { data: user ?? null, isLoading: false };
}

export function useRoles() {
  const user = useAuthStore((s) => s.user);
  return { data: user?.roles ?? [], isLoading: false };
}

export function useIsAdmin() {
  const { data: roles } = useRoles();
  return (roles ?? []).includes("admin");
}

export const formatDate = (iso: string) =>
  new Date(iso).toLocaleDateString("en-GB", { day: "2-digit", month: "short", year: "numeric" });