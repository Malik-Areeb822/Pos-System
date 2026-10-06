import { useEffect } from "react";
import { useLicenseStore } from "@/lib/license-store";
import { checkSystemLock } from "@/features/license/api";
import { LockScreen } from "./LockScreen";
import { KeyComboListener } from "./KeyComboListener";

export function LicenseGate({ children }: { children: React.ReactNode }) {
  const { status, isLoading, setStatus, setLoading } = useLicenseStore();

  useEffect(() => {
    checkSystemLock()
      .then(setStatus)
      .catch(() => {
        // On error, assume locked (fail-closed)
        setStatus({ is_locked: true });
      });
  }, [setStatus]);

  // Still loading — show nothing (app starts with splash/loading)
  if (isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background">
        <div className="text-sm text-muted-foreground">Loading...</div>
      </div>
    );
  }

  // Locked — show the lock screen
  if (status?.is_locked) {
    return (
      <>
        <KeyComboListener />
        <LockScreen />
      </>
    );
  }

  // Unlocked — render children normally, but keep the combo listener active
  return (
    <>
      <KeyComboListener />
      {children}
    </>
  );
}
