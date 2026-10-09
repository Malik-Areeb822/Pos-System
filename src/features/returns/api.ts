import { api } from "@/lib/api-client";
import { useQuery, useMutation, useQueryClient, type QueryClient } from "@tanstack/react-query";

export type Return = {
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
};

export type CreateReturnInput = {
  invoice_id: string;
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
  reason: string;
};

// No `line_total` anywhere in a request: the backend derives it from
// `quantity * unit_price`, so a tampered payload cannot overstate the credit.
export type CreateReturnLineInput = {
  product_id: string | null;
  product_name: string;
  quantity: number;
  unit: string;
  unit_price: number;
};

export type CreateReturnsBulkInput = {
  invoice_id: string;
  reason: string;
  lines: CreateReturnLineInput[];
};

export function useReturns() {
  return useQuery({
    queryKey: ["returns"],
    queryFn: () => api.returns.list(),
  });
}

export function useCreateReturn() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateReturnInput) => api.returns.create(input),
    onSuccess: () => invalidateAfterReturn(queryClient),
  });
}

/** The whole return goes to the backend as ONE call — one transaction, so a
 *  rejected line can never leave an earlier line already applied. */
export function useCreateReturnsBulk() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateReturnsBulkInput) => api.returns.createBulk(input),
    onSuccess: () => invalidateAfterReturn(queryClient),
  });
}

function invalidateAfterReturn(queryClient: QueryClient) {
  queryClient.invalidateQueries({ queryKey: ["returns"] });
  queryClient.invalidateQueries({ queryKey: ["invoices"] });
  queryClient.invalidateQueries({ queryKey: ["products"] });
  queryClient.invalidateQueries({ queryKey: ["dashboard"] });
  queryClient.invalidateQueries({ queryKey: ["customers"] });
}
