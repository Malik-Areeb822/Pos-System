import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "./auth-store";
import { BUSINESS } from "./business";

export interface ApiError extends Error {
  code?: string;
  status?: number;
}

export interface CircuitBreakerState {
  failures: number;
  lastFailure: number;
  state: "closed" | "open" | "half-open";
}

function createCircuitBreaker() {
  const state: CircuitBreakerState = {
    failures: 0,
    lastFailure: 0,
    state: "closed",
  };

  const THRESHOLD = 5;
  const TIMEOUT = 30_000;

  return {
    async call<T>(fn: () => Promise<T>): Promise<T> {
      if (state.state === "open") {
        if (Date.now() - state.lastFailure > TIMEOUT) {
          state.state = "half-open";
        } else {
          const error = new Error(
            "Circuit breaker open - service temporarily unavailable",
          ) as ApiError;
          error.code = "CIRCUIT_OPEN";
          throw error;
        }
      }

      try {
        const result = await fn();
        this.onSuccess();
        return result;
      } catch (error) {
        this.onFailure();
        throw error;
      }
    },
    onSuccess() {
      state.failures = 0;
      state.state = "closed";
    },
    onFailure() {
      state.failures += 1;
      state.lastFailure = Date.now();
      if (state.failures >= THRESHOLD) {
        state.state = "open";
      }
    },
    getState() {
      return { ...state };
    },
    reset() {
      state.failures = 0;
      state.lastFailure = 0;
      state.state = "closed";
    },
  };
}

const circuitBreakers = new Map<string, ReturnType<typeof createCircuitBreaker>>();

function getCircuitBreaker(key: string) {
  if (!circuitBreakers.has(key)) {
    circuitBreakers.set(key, createCircuitBreaker());
  }
  return circuitBreakers.get(key)!;
}

function getAuthHeader(): string | undefined {
  const { token } = useAuthStore.getState();
  return token ? `Bearer ${token}` : undefined;
}

export async function apiInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
  options?: { feature?: string },
): Promise<T> {
  const feature = options?.feature || "default";
  const breaker = getCircuitBreaker(feature);

  return breaker.call(async () => {
    const payload: Record<string, unknown> = { ...args };
    const authHeader = getAuthHeader();
    if (authHeader) {
      payload.authHeader = authHeader;
    }
    try {
      return await invoke<T>(command, payload);
    } catch (e) {
      throw e instanceof Error ? e : new Error(typeof e === "string" ? e : JSON.stringify(e));
    }
  });
}

export function resetCircuitBreaker(feature: string) {
  const breaker = circuitBreakers.get(feature);
  if (breaker) breaker.reset();
}

export function getCircuitBreakerState(feature: string): CircuitBreakerState | null {
  const breaker = circuitBreakers.get(feature);
  return breaker ? breaker.getState() : null;
}

export interface ApiClient {
  products: ProductsApi;
  customers: CustomersApi;
  invoices: InvoicesApi;
  returns: ReturnsApi;
  reports: ReportsApi;
  cashiers: CashiersApi;
  auth: AuthApi;
  backups: BackupsApi;
  suppliers: SuppliersApi;
}

export interface ProductsApi {
  list(category?: string): Promise<Product[]>;
  get(id: string): Promise<Product | null>;
  create(input: CreateProductInput): Promise<Product>;
  update(id: string, input: Partial<CreateProductInput>): Promise<Product>;
  delete(id: string): Promise<void>;
  import(csvContent: string): Promise<ImportResult>;
}

export interface CustomersApi {
  list(): Promise<Customer[]>;
  get(id: string): Promise<Customer | null>;
  create(input: CreateCustomerInput): Promise<Customer>;
  update(id: string, input: Partial<CreateCustomerInput>): Promise<Customer>;
  delete(id: string): Promise<void>;
}

export interface MarkPaidInput {
  id: string;
  amount: number;
  payment_method: "cash" | "bank" | "credit";
}

export interface ListInvoicesParams {
  query?: string;
}

export interface InvoicesApi {
  list(params?: ListInvoicesParams): Promise<Invoice[]>;
  get(id: string): Promise<Invoice | null>;
  getWithItems(id: string): Promise<{ invoice: Invoice; items: InvoiceItem[] } | null>;
  create(input: CreateInvoiceInput): Promise<Invoice>;
  markPaid(input: MarkPaidInput): Promise<Invoice>;
  printReceipt(id: string): Promise<void>;
  printInvoicePdf(id: string): Promise<string>;
}

export interface ReturnsApi {
  list(): Promise<Return[]>;
  create(input: CreateReturnInput): Promise<Return>;
}

export interface ReportsApi {
  getDashboard(): Promise<DashboardStats>;
  getSales(params: ReportParams): Promise<SalesReportItem[]>;
  getInventory(): Promise<InventoryReportItem[]>;
}

export interface CashiersApi {
  list(): Promise<StaffProfile[]>;
  approve(id: string): Promise<void>;
  reject(id: string): Promise<void>;
  suspend(id: string): Promise<void>;
  resetPassword(id: string, newPassword: string): Promise<void>;
}

export interface AuthApi {
  login(email: string, password: string): Promise<AuthResult>;
  register(input: RegisterInput): Promise<AuthResult>;
  logout(): Promise<void>;
  me(): Promise<User | null>;
  checkAdminExists(): Promise<boolean>;
}

export interface BackupInfo {
  path: string;
  name: string;
  size: number;
  created: number;
}

export interface BackupsApi {
  export(): Promise<string>;
  import(backupPath: string): Promise<void>;
  list(): Promise<BackupInfo[]>;
}

export interface Product {
  id: string;
  name: string;
  sku: string | null;
  category: "marble" | "tiles" | "chips" | "sanitary";
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
}

export interface CreateProductInput {
  name: string;
  sku?: string | null;
  category: "marble" | "tiles" | "chips" | "sanitary";
  description: string;
  color?: string | null;
  size?: string | null;
  finish?: string | null;
  company?: string | null;
  unit: string;
  price: number;
  pieces_per_carton?: number | null;
  area_per_tile?: number | null;
  stock_qty: number;
  low_stock_threshold: number;
  image_url?: string | null;
  is_published?: boolean;
}

export interface ImportResult {
  imported: number;
  errors: string[];
}

export interface Customer {
  id: string;
  name: string;
  phone: string | null;
  email: string | null;
  address: string | null;
  outstanding_balance: number;
  created_at: string;
}

export interface CreateCustomerInput {
  name: string;
  phone?: string;
  email?: string;
  address?: string;
}

export interface Invoice {
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
}

export interface InvoiceItem {
  id: string;
  invoice_id: string;
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
  total_area: number | null;
}

export interface CreateInvoiceInput {
  customer_id: string | null;
  customer_name: string;
  items: CreateInvoiceItemInput[];
  subtotal: number;
  discount?: number;
  total: number;
  amount_paid?: number;
  payment_method: "cash" | "bank" | "credit";
  notes?: string;
  delivery_date?: string;
}

export interface CreateInvoiceItemInput {
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
  total_area?: number | null;
}

export interface Return {
  id: string;
  invoice_id: string;
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
  reason: string;
  processed_by: string;
  created_at: string;
}

export interface CreateReturnInput {
  invoice_id: string;
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
  reason: string;
}

// Wire-shape truth: these mirror the serde structs in src-tauri/src/commands/reports.rs
export interface DashboardStats {
  total_sales_today: number;
  total_invoices_today: number;
  low_stock_count: number;
  outstanding_balance: number;
}

export interface ReportParams {
  from?: string;
  to?: string;
}

export interface SalesReportItem {
  date: string;
  total_sales: number;
  invoice_count: number;
  cash_sales: number;
  credit_sales: number;
  bank_sales: number;
}

export interface InventoryReportItem {
  id: string;
  name: string;
  sku: string | null;
  category: string;
  stock_qty: number;
  low_stock_threshold: number;
  unit: string;
  price: number;
  value: number;
}

export interface StaffProfile {
  id: string;
  full_name: string;
  email: string | null;
  phone: string | null;
  employee_id: string | null;
  status: "pending" | "approved" | "rejected" | "suspended";
  created_at: string;
  approved_at: string | null;
  rejected_at: string | null;
}

export interface User {
  id: string;
  email: string;
  full_name: string;
  phone: string | null;
  employee_id: string | null;
  roles: ("admin" | "cashier")[];
  status: "pending" | "approved" | "rejected" | "suspended";
}

export interface AuthResult {
  user: User;
  token: string;
}

export interface RegisterInput {
  email: string;
  password: string;
  full_name: string;
  phone?: string;
  employee_id?: string;
}

export interface Supplier {
  id: string;
  name: string;
  phone: string | null;
  email: string | null;
  company: string | null;
  address: string | null;
  notes: string | null;
  outstanding_balance: number;
  created_at: string;
  updated_at: string;
}

export interface CreateSupplierInput {
  name: string;
  phone?: string;
  email?: string;
  company?: string;
  address?: string;
  notes?: string;
}

export interface SupplierPurchase {
  id: string;
  purchase_no: string;
  supplier_id: string;
  supplier_name: string;
  subtotal: number;
  discount: number;
  total: number;
  amount_paid: number;
  payment_method: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface SupplierPurchaseItem {
  id: string;
  purchase_id: string;
  description: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
  created_at: string;
}

export interface CreateSupplierPurchaseItemInput {
  description: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
}

export interface CreateSupplierPurchaseInput {
  supplier_id: string;
  supplier_name: string;
  subtotal: number;
  discount: number;
  total: number;
  amount_paid?: number;
  payment_method: string;
  notes?: string;
  items: CreateSupplierPurchaseItemInput[];
}

export interface ListSupplierPurchasesParams {
  limit?: number;
  offset?: number;
  query?: string;
  supplier_id?: string;
}

export interface SuppliersApi {
  list(): Promise<Supplier[]>;
  get(id: string): Promise<Supplier | null>;
  create(input: CreateSupplierInput): Promise<Supplier>;
  update(id: string, input: Partial<CreateSupplierInput>): Promise<Supplier>;
  delete(id: string): Promise<void>;
  listPurchases(params?: ListSupplierPurchasesParams): Promise<SupplierPurchase[]>;
  getPurchaseWithItems(id: string): Promise<{ purchase: SupplierPurchase; items: SupplierPurchaseItem[] } | null>;
  createPurchase(input: CreateSupplierPurchaseInput): Promise<SupplierPurchase>;
  markPurchasePaid(input: { id: string; amount: number; payment_method: string }): Promise<SupplierPurchase>;
}

export const api: ApiClient = {
  products: {
    list: (category) =>
      apiInvoke("list_products", { input: category ? { category } : {} }, { feature: "products" }),
    get: (id) => apiInvoke("get_product", { id }, { feature: "products" }),
    create: (input) => apiInvoke("create_product", { input }, { feature: "products" }),
    update: (id, input) =>
      apiInvoke("update_product", { input: { ...input, id } }, { feature: "products" }),
    delete: (id) => apiInvoke("delete_product", { id }, { feature: "products" }),
    import: (csvContent) => apiInvoke("import_products", { csvContent }, { feature: "products" }),
  },
  customers: {
    list: () => apiInvoke("list_customers", {}, { feature: "customers" }),
    get: (id) => apiInvoke("get_customer", { id }, { feature: "customers" }),
    create: (input) => apiInvoke("create_customer", { input }, { feature: "customers" }),
    update: (id, input) =>
      apiInvoke("update_customer", { input: { ...input, id } }, { feature: "customers" }),
    delete: (id) => apiInvoke("delete_customer", { id }, { feature: "customers" }),
  },
  invoices: {
    list: (params?: ListInvoicesParams) =>
      apiInvoke("list_invoices", { input: params ?? {} }, { feature: "invoices" }),
    get: (id) => apiInvoke("get_invoice", { id }, { feature: "invoices" }),
    getWithItems: async (id) => {
      const res = await apiInvoke<[Invoice, InvoiceItem[]] | null>(
        "get_invoice_with_items",
        { id },
        { feature: "invoices" },
      );
      if (!res) return null;
      const [invoice, items] = res;
      return { invoice, items };
    },
    create: (input) => apiInvoke("create_invoice", { input }, { feature: "invoices" }),
    markPaid: (input) => apiInvoke("mark_invoice_paid", { input }, { feature: "invoices" }),
    printReceipt: (id) =>
      apiInvoke(
        "print_receipt",
        {
          invoiceId: id,
          business: {
            name: BUSINESS.name,
            address: BUSINESS.address,
            phone: BUSINESS.phone,
          },
        },
        { feature: "invoices" },
      ),
    printInvoicePdf: (id) =>
      apiInvoke<string>("print_invoice_pdf", { invoiceId: id }, { feature: "invoices" }),
  },
  returns: {
    list: () => apiInvoke("list_returns", { input: {} }, { feature: "returns" }),
    create: (input) => apiInvoke("create_return", { input }, { feature: "returns" }),
  },
  reports: {
    getDashboard: () => apiInvoke("get_dashboard", {}, { feature: "reports" }),
    getSales: (params) =>
      apiInvoke(
        "get_sales_report",
        { input: { from_date: params.from ?? null, to_date: params.to ?? null } },
        { feature: "reports" },
      ),
    getInventory: () => apiInvoke("get_inventory_report", {}, { feature: "reports" }),
  },
  cashiers: {
    list: () => apiInvoke("list_cashiers", {}, { feature: "cashiers" }),
    approve: (id) => apiInvoke("approve_cashier", { input: { id } }, { feature: "cashiers" }),
    reject: (id) => apiInvoke("reject_cashier", { input: { id } }, { feature: "cashiers" }),
    suspend: (id) => apiInvoke("suspend_cashier", { input: { id } }, { feature: "cashiers" }),
    resetPassword: (id, newPassword) =>
      apiInvoke(
        "reset_password",
        { input: { id, new_password: newPassword } },
        { feature: "cashiers" },
      ),
  },
  auth: {
    login: (email, password) =>
      apiInvoke("login", { input: { email, password } }, { feature: "auth" }),
    register: (input) => apiInvoke("register", { input }, { feature: "auth" }),
    logout: () => apiInvoke("logout", {}, { feature: "auth" }),
    me: () => apiInvoke("me", {}, { feature: "auth" }),
    checkAdminExists: async () => {
      const res = await apiInvoke<{ exists: boolean }>(
        "check_admin_exists",
        {},
        { feature: "auth" },
      );
      return res?.exists ?? false;
    },
  },
  backups: {
    export: () => apiInvoke<string>("export_database", {}, { feature: "backups" }),
    import: (backupPath) => apiInvoke("import_database", { backupPath }, { feature: "backups" }),
    list: () => apiInvoke("list_backups", {}, { feature: "backups" }),
  },
  suppliers: {
    list: () => apiInvoke("list_suppliers", {}, { feature: "suppliers" }),
    get: (id) => apiInvoke("get_supplier", { id }, { feature: "suppliers" }),
    create: (input) => apiInvoke("create_supplier", { input }, { feature: "suppliers" }),
    update: (id, input) =>
      apiInvoke("update_supplier", { input: { ...input, id } }, { feature: "suppliers" }),
    delete: (id) => apiInvoke("delete_supplier", { id }, { feature: "suppliers" }),
    listPurchases: (params?: ListSupplierPurchasesParams) =>
      apiInvoke("list_supplier_purchases", { input: params ?? {} }, { feature: "suppliers" }),
    getPurchaseWithItems: async (id) => {
      const res = await apiInvoke<[SupplierPurchase, SupplierPurchaseItem[]] | null>(
        "get_supplier_purchase_with_items",
        { id },
        { feature: "suppliers" },
      );
      if (!res) return null;
      const [purchase, items] = res;
      return { purchase, items };
    },
    createPurchase: (input) =>
      apiInvoke("create_supplier_purchase", { input }, { feature: "suppliers" }),
    markPurchasePaid: (input) =>
      apiInvoke("mark_supplier_purchase_paid", { input }, { feature: "suppliers" }),
  },
};
