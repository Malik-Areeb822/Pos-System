import { api, type ReportParams } from "@/lib/api-client";
import { useQuery } from "@tanstack/react-query";

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
