// POS SYSTEM — ACTIVE. Entry point: this login screen is served at "/".
import { createFileRoute, useNavigate, Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuthStore } from "@/lib/auth-store";
import { useLogin, useRegister, useCheckAdminExists } from "@/features/auth/api";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "Staff Sign In — City Tiles" },
      {
        name: "description",
        content: "Sign in to the City Tiles point-of-sale and inventory dashboard.",
      },
      { property: "og:title", content: "Staff Sign In — City Tiles" },
      { property: "og:description", content: "Point-of-sale and inventory dashboard access." },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: AuthPage,
});

function AuthPage() {
  const navigate = useNavigate();
  const { isAuthenticated } = useAuthStore();
  const [mode, setMode] = useState<"signin" | "signup">("signin");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [fullName, setFullName] = useState("");
  const [phone, setPhone] = useState("");
  const [employeeId, setEmployeeId] = useState("");

  const { data: adminExists } = useCheckAdminExists();
  const isAdminSetup = mode === "signup" && adminExists === false;

  const loginMutation = useLogin();
  const registerMutation = useRegister();

  useEffect(() => {
    if (isAuthenticated) {
      navigate({ to: "/admin", replace: true });
    }
  }, [isAuthenticated, navigate]);

  useEffect(() => {
    if (adminExists === false) {
      setMode("signup");
    } else if (adminExists === true) {
      setMode("signin");
    }
  }, [adminExists]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    try {
      if (mode === "signup") {
        await registerMutation.mutateAsync({
          email,
          password,
          full_name: fullName,
          phone,
          employee_id: employeeId,
        });
        if (!isAdminSetup) {
          toast.success(
            "Your cashier registration request has been submitted. Please wait for Admin approval."
          );
        }
      } else {
        await loginMutation.mutateAsync({ email, password });
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Sign in failed");
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-[--marble-black] px-5 py-16">
      <div className="w-full max-w-md border border-border/40 bg-card p-8 md:p-10 shadow-stone">
        <Link to="/website" className="eyebrow text-brass">
          City Tiles
        </Link>
        <h1 className="mt-5 text-3xl">
          {mode === "signin"
            ? "Staff sign in"
            : isAdminSetup
            ? "First-time admin setup"
            : "Register as cashier"}
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          {mode === "signin"
            ? "Access to the point-of-sale, inventory and sales reports."
            : isAdminSetup
            ? "No admin account exists yet. This one-time setup creates the single owner account."
            : "Submit a cashier request. The admin must approve it before you can use the POS."}
        </p>

        <form onSubmit={handleSubmit} className="mt-8 space-y-4">
          {mode === "signup" && (
            <>
              <div className="space-y-2">
                <Label htmlFor="fullName">Full name</Label>
                <Input
                  id="fullName"
                  value={fullName}
                  onChange={(e) => setFullName(e.target.value)}
                  required
                  maxLength={100}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="phone">Phone (optional)</Label>
                <Input
                  id="phone"
                  value={phone}
                  onChange={(e) => setPhone(e.target.value)}
                  maxLength={30}
                />
              </div>
              {!isAdminSetup && (
                <div className="space-y-2">
                  <Label htmlFor="employeeId">Employee ID (optional)</Label>
                  <Input
                    id="employeeId"
                    value={employeeId}
                    onChange={(e) => setEmployeeId(e.target.value)}
                    maxLength={50}
                  />
                </div>
              )}
            </>
          )}
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              maxLength={255}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="password">Password</Label>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={6}
            />
          </div>
          <Button
            type="submit"
            variant="brass"
            size="xl"
            className="w-full"
            disabled={loginMutation.isPending || registerMutation.isPending}
          >
            {loginMutation.isPending || registerMutation.isPending
              ? "Please wait…"
              : mode === "signin"
              ? "Sign in"
              : isAdminSetup
              ? "Create admin account"
              : "Submit cashier request"}
          </Button>
        </form>

        <div className="my-6 flex items-center gap-3 text-xs text-muted-foreground">
          <span className="h-px flex-1 bg-border" /> or <span className="h-px flex-1 bg-border" />
        </div>

        <Button variant="stone" size="xl" className="w-full" disabled>
          Continue with Google (offline mode)
        </Button>

        <button
          type="button"
          onClick={() => setMode(mode === "signin" ? "signup" : "signin")}
          className="mt-6 w-full text-sm text-muted-foreground hover:text-brass"
        >
          {mode === "signin"
            ? adminExists === false
              ? "No admin yet? Run first-time admin setup"
              : "No account yet? Register as cashier"
            : "Already have an account? Sign in"}
        </button>
      </div>
    </div>
  );
}