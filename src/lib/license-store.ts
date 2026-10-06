import { create } from "zustand";

export interface LockStatus {
  is_locked: boolean;
}

interface LicenseState {
  status: LockStatus | null;
  isLoading: boolean;
  setStatus: (status: LockStatus) => void;
  setLoading: (loading: boolean) => void;
}

export const useLicenseStore = create<LicenseState>((set) => ({
  status: null,
  isLoading: true,
  setStatus: (status) => set({ status, isLoading: false }),
  setLoading: (isLoading) => set({ isLoading }),
}));
