import { api } from "@/lib/api-client";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import type {
  Supplier,
  CreateSupplierInput,
  SupplierPurchase,
  CreateSupplierPurchaseInput,
  ListSupplierPurchasesParams,
} from "@/lib/api-client";

export function useSuppliers() {
  return useQuery({
    queryKey: ["suppliers"],
    queryFn: () => api.suppliers.list(),
  });
}

export function useSupplier(id: string) {
  return useQuery({
    queryKey: ["supplier", id],
    queryFn: () => api.suppliers.get(id),
    enabled: !!id,
  });
}

export function useSupplierPurchases(params?: ListSupplierPurchasesParams) {
  return useQuery({
    queryKey: ["supplier-purchases", params],
    queryFn: () => api.suppliers.listPurchases(params),
  });
}

export function useSupplierPurchase(id: string) {
  return useQuery({
    queryKey: ["supplier-purchase", id],
    queryFn: () => api.suppliers.getPurchaseWithItems(id),
    enabled: !!id,
  });
}

export function useCreateSupplier() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateSupplierInput) => api.suppliers.create(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["suppliers"] });
    },
  });
}

export function useUpdateSupplier() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: Partial<CreateSupplierInput> }) =>
      api.suppliers.update(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["suppliers"] });
    },
  });
}

export function useDeleteSupplier() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.suppliers.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["suppliers"] });
      queryClient.invalidateQueries({ queryKey: ["supplier-purchases"] });
    },
  });
}

export function useCreateSupplierPurchase() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateSupplierPurchaseInput) => api.suppliers.createPurchase(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["supplier-purchases"] });
      queryClient.invalidateQueries({ queryKey: ["suppliers"] });
    },
  });
}

export function useMarkSupplierPurchasePaid() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { id: string; amount: number; payment_method: string }) =>
      api.suppliers.markPurchasePaid(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["supplier-purchases"] });
      queryClient.invalidateQueries({ queryKey: ["suppliers"] });
    },
  });
}
