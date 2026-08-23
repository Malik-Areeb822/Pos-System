import { useNavigate } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import { ShieldAlert, Clock, XCircle } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { useAuthStore } from "@/lib/auth-store";
import { api } from "@/lib/api-client";

function Notice({ icon: Icon, title, body }: { icon: typeof Clock; title: string; body: string }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  async function signOut() {
    await queryClient.cancelQueries();
    queryClient.clear();
    useAuthStore.getState().clearAuth();
    navigate({ to: "/", replace: true });
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/40 px-5">
      <div className="w-full max-w-md rounded-lg border border-border bg-card p-8 text-center">
        <Icon className="mx-auto size-8 text-brass" />
        <h1 className="mt-4 text-xl font-semibold">{title}</h1>
        <p className="mt-2 text-sm text-muted-foreground">{body}</p>
        <Button variant="outline" className="mt-6" onClick={signOut}>
          Sign out
        </Button>
      </div>
    </div>
  );
}

export function AccessGate({ children }: { children: ReactNode }) {
  const { user, isAuthenticated } = useAuthStore();

  if (!isAuthenticated) {
    return <div className="p-10 text-sm text-muted-foreground">Checking your account…</div>;
  }

  const status = user?.status ?? "pending";

  if (status === "pending") {
    return (
      <Notice
        icon={Clock}
        title="Awaiting admin approval"
        body="Your account is waiting for Admin approval. You will be able to sign in to the POS once the admin approves your request."
      />
    );
  }
  if (status === "rejected") {
    return (
      <Notice
        icon={XCircle}
        title="Registration rejected"
        body="Your cashier registration request was rejected by the admin. Please contact the shop admin if you believe this is a mistake."
      />
    );
  }
  if (status === "suspended") {
    return (
      <Notice
        icon={ShieldAlert}
        title="Account suspended"
        body="Your account has been suspended by the admin. Contact the admin to have it reactivated."
      />
    );
  }

  return <>{children}</>;
}