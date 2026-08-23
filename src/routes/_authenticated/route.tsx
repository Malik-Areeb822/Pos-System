import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";
import { useAuthStore } from "@/lib/auth-store";
import { AccessGate } from "@/components/admin/AccessGate";

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
      <Outlet />
    </AccessGate>
  ),
});