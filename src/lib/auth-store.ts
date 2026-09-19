import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

export interface User {
  id: string;
  email: string;
  full_name: string;
  phone: string | null;
  employee_id: string | null;
  roles: ("admin" | "cashier")[];
  status: "pending" | "approved" | "rejected" | "suspended";
}

interface AuthState {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;

  setAuth: (user: User, token: string) => void;
  clearAuth: () => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  updateUser: (user: Partial<User>) => void;
  hasRole: (role: "admin" | "cashier") => boolean;
  isAdmin: () => boolean;
}

const STORAGE_KEY = "moon-pipe-auth";

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      token: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,

      setAuth: (user, token) =>
        set({
          user,
          token,
          isAuthenticated: true,
          error: null,
        }),

      clearAuth: () =>
        set({
          user: null,
          token: null,
          isAuthenticated: false,
          error: null,
        }),

      setLoading: (isLoading) => set({ isLoading }),

      setError: (error) => set({ error }),

      updateUser: (updates) =>
        set((state) => ({
          user: state.user ? { ...state.user, ...updates } : null,
        })),

      hasRole: (role) => {
        const { user } = get();
        return user?.roles.includes(role) ?? false;
      },

      isAdmin: () => {
        const { user } = get();
        return user?.roles.includes("admin") ?? false;
      },
    }),
    {
      name: STORAGE_KEY,
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        user: state.user,
        token: state.token,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
);