import { api } from "@/lib/api-client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useAuthStore } from "@/lib/auth-store";
import { useNavigate } from "@tanstack/react-router";

export type User = {
  id: string;
  email: string;
  full_name: string;
  phone: string | null;
  employee_id: string | null;
  roles: ("admin" | "cashier")[];
  status: "pending" | "approved" | "rejected" | "suspended";
};

export type AuthResult = {
  user: User;
  token: string;
};

export type RegisterInput = {
  email: string;
  password: string;
  full_name: string;
  phone?: string;
  employee_id?: string;
};

export function useLogin() {
  const queryClient = useQueryClient();
  const { setAuth, setLoading, setError } = useAuthStore();
  const navigate = useNavigate();

  return useMutation({
    mutationFn: ({ email, password }: { email: string; password: string }) =>
      api.auth.login(email, password),
    onMutate: () => {
      setLoading(true);
      setError(null);
    },
    onSuccess: (data) => {
      setAuth(data.user, data.token);
      queryClient.invalidateQueries({ queryKey: ["session-user"] });
      queryClient.invalidateQueries({ queryKey: ["roles"] });
      queryClient.invalidateQueries({ queryKey: ["my-profile"] });
      navigate({ to: "/admin", replace: true });
    },
    onError: (err: Error) => {
      setError(err.message);
    },
    onSettled: () => {
      setLoading(false);
    },
  });
}

export function useRegister() {
  const queryClient = useQueryClient();
  const { setAuth, setLoading, setError } = useAuthStore();
  const navigate = useNavigate();

  return useMutation({
    mutationFn: (input: RegisterInput) => api.auth.register(input),
    onMutate: () => {
      setLoading(true);
      setError(null);
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ["admin-exists"] });
      if (data.token) {
        setAuth(data.user, data.token);
        queryClient.invalidateQueries({ queryKey: ["session-user"] });
        queryClient.invalidateQueries({ queryKey: ["roles"] });
        queryClient.invalidateQueries({ queryKey: ["my-profile"] });
        navigate({ to: "/admin", replace: true });
      }
    },
    onError: (err: Error) => {
      setError(err.message);
    },
    onSettled: () => {
      setLoading(false);
    },
  });
}

export function useLogout() {
  const { clearAuth } = useAuthStore();
  const queryClient = useQueryClient();
  const navigate = useNavigate();

  return useMutation({
    mutationFn: () => api.auth.logout(),
    onSuccess: () => {
      clearAuth();
      queryClient.clear();
      navigate({ to: "/", replace: true });
    },
  });
}

export function useMe() {
  return useQuery({
    queryKey: ["auth-me"],
    queryFn: () => api.auth.me(),
    retry: false,
  });
}

export function useCheckAdminExists() {
  return useQuery({
    queryKey: ["admin-exists"],
    queryFn: () => api.auth.checkAdminExists(),
    staleTime: 0,
    refetchOnMount: "always",
  });
}

export function useRoles() {
  const user = useAuthStore((s) => s.user);
  return { data: user?.roles ?? [], isLoading: false };
}