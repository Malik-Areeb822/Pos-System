import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

export type TauriEventName = "invoices:changed" | "customers:changed" | "products:changed" | "cashiers:changed";

export interface EventSubscription {
  unlisten: () => Promise<void>;
}

const subscriptions = new Map<TauriEventName, UnlistenFn>();

export async function subscribeToEvent(eventName: TauriEventName, handler: () => void): Promise<EventSubscription> {
  if (subscriptions.has(eventName)) {
    await subscriptions.get(eventName)!();
  }
  const unlisten = await listen(eventName, handler);
  subscriptions.set(eventName, unlisten);
  return { unlisten };
}

export async function unsubscribeFromEvent(eventName: TauriEventName): Promise<void> {
  const unlisten = subscriptions.get(eventName);
  if (unlisten) {
    await unlisten();
    subscriptions.delete(eventName);
  }
}

export async function unsubscribeAll(): Promise<void> {
  for (const [, unlisten] of subscriptions) {
    await unlisten();
  }
  subscriptions.clear();
}

export function useInvoiceRealtime(invoiceId?: string) {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlistenInvoices: UnlistenFn | null = null;
    let unlistenCustomers: UnlistenFn | null = null;

    const setup = async () => {
      unlistenInvoices = await listen("invoices:changed", () => {
        queryClient.invalidateQueries({ queryKey: ["invoices"] });
        queryClient.invalidateQueries({ queryKey: ["customers"] });
        queryClient.invalidateQueries({ queryKey: ["dashboard"] });
        if (invoiceId) queryClient.invalidateQueries({ queryKey: ["invoice", invoiceId] });
      });

      unlistenCustomers = await listen("customers:changed", () => {
        queryClient.invalidateQueries({ queryKey: ["customers"] });
        queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      });
    };

    setup();

    return () => {
      unlistenInvoices?.();
      unlistenCustomers?.();
    };
  }, [queryClient, invoiceId]);
}

export function useProductsRealtime() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen("products:changed", () => {
        queryClient.invalidateQueries({ queryKey: ["products"] });
        queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      });
    };

    setup();

    return () => {
      unlisten?.();
    };
  }, [queryClient]);
}

export function useCashiersRealtime() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen("cashiers:changed", () => {
        queryClient.invalidateQueries({ queryKey: ["cashiers"] });
        queryClient.invalidateQueries({ queryKey: ["staff"] });
      });
    };

    setup();

    return () => {
      unlisten?.();
    };
  }, [queryClient]);
}

export async function emitEvent(eventName: TauriEventName): Promise<void> {
  const { emit } = await import("@tauri-apps/api/event");
  await emit(eventName);
}