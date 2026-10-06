import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAuthStore } from "@/lib/auth-store";
import { AccessGate } from "@/components/admin/AccessGate";

function GlobalRealtimeListener() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlistenFns: (() => void)[] = [];

    listen("invoices:changed", () => {
      queryClient.invalidateQueries({ queryKey: ["invoices"] });
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      // Sales totals are derived from invoices too; nothing else refreshes them.
      queryClient.invalidateQueries({ queryKey: ["sales-report"] });
    }).then((fn) => unlistenFns.push(fn));

    listen("products:changed", () => {
      queryClient.invalidateQueries({ queryKey: ["products"] });
      queryClient.invalidateQueries({ queryKey: ["pos-products"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      queryClient.invalidateQueries({ queryKey: ["inventory-report"] });
    }).then((fn) => unlistenFns.push(fn));

    listen("customers:changed", () => {
      queryClient.invalidateQueries({ queryKey: ["customers"] });
      // Separate key (pos/api.ts) — invalidating ["customers"] does not touch it.
      queryClient.invalidateQueries({ queryKey: ["pos-customers"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard"] });
    }).then((fn) => unlistenFns.push(fn));

    listen("suppliers:changed", () => {
      queryClient.invalidateQueries({ queryKey: ["suppliers"] });
      queryClient.invalidateQueries({ queryKey: ["supplier-purchases"] });
    }).then((fn) => unlistenFns.push(fn));

    listen("cashiers:changed", () => {
      queryClient.invalidateQueries({ queryKey: ["cashiers"] });
    }).then((fn) => unlistenFns.push(fn));

    // A restore replaced every row under the app's feet: every cached query
    // is now describing a database that no longer exists. Refetch all of them.
    listen("database:restored", () => {
      queryClient.invalidateQueries();
    }).then((fn) => unlistenFns.push(fn));

    return () => {
      for (const unlisten of unlistenFns) unlisten();
    };
  }, [queryClient]);

  return null;
}

export const Route = createFileRoute("/_authenticated")({
  ssr: false,
  beforeLoad: () => {
    const { isAuthenticated, token } = useAuthStore.getState();
    if (!isAuthenticated || !token) {
      throw redirect({ to: "/" });
    }
  },
  component: () => (
    <AccessGate>
      <GlobalRealtimeListener />
      <Outlet />
    </AccessGate>
  ),
});
