import { api } from "@/lib/api-client";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

export type StaffProfile = {
  id: string;
  full_name: string;
  email: string | null;
  phone: string | null;
  employee_id: string | null;
  status: "pending" | "approved" | "rejected" | "suspended";
  created_at: string;
  approved_at: string | null;
  rejected_at: string | null;
};

export function useCashiers() {
  return useQuery({
    queryKey: ["cashiers"],
    queryFn: () => api.cashiers.list(),
  });
}

export function useApproveCashier() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.cashiers.approve(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cashiers"] });
    },
  });
}

export function useRejectCashier() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.cashiers.reject(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cashiers"] });
    },
  });
}

export function useSuspendCashier() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.cashiers.suspend(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cashiers"] });
    },
  });
}

export function useResetCashierPassword() {
  return useMutation({
    mutationFn: (id: string) => api.cashiers.resetPassword(id),
  });
}