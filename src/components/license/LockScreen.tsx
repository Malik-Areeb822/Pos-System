import { useState } from "react";
import { toast } from "sonner";
import { Lock, Loader2, KeyRound } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useLicenseStore } from "@/lib/license-store";
import { unlockWithPassword } from "@/features/license/api";

export function LockScreen() {
  const setStatus = useLicenseStore((s) => s.setStatus);
  const [password, setPassword] = useState("");
  const [isUnlocking, setIsUnlocking] = useState(false);

  async function handleUnlock(e: React.FormEvent) {
    e.preventDefault();
    if (!password.trim()) {
      toast.error("Please enter an unlock password");
      return;
    }
    setIsUnlocking(true);
    try {
      const result = await unlockWithPassword(password.trim());
      setStatus(result);
      toast.success("System unlocked successfully");
    } catch {
      toast.error("Invalid unlock password");
    } finally {
      setIsUnlocking(false);
      setPassword("");
    }
  }

  return (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-background px-5">
      <div className="w-full max-w-md rounded-lg border border-border bg-card p-8 md:p-10 shadow-lg">
        <div className="text-center">
          <Lock className="mx-auto size-10 text-primary" />
          <h1 className="mt-4 text-2xl font-semibold">System Locked</h1>
          <p className="mt-2 text-sm text-muted-foreground">Contact AZ Solutions</p>
        </div>

        <form onSubmit={handleUnlock} className="mt-8 space-y-4">
          <div className="space-y-2">
            <Label htmlFor="unlock-password">Unlock Password</Label>
            <Input
              id="unlock-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter unlock code"
              autoFocus
              disabled={isUnlocking}
            />
          </div>

          <Button
            type="submit"
            size="lg"
            className="w-full"
            disabled={isUnlocking || !password.trim()}
          >
            {isUnlocking ? (
              <Loader2 className="mr-2 size-4 animate-spin" />
            ) : (
              <KeyRound className="mr-2 size-4" />
            )}
            Unlock
          </Button>
        </form>
      </div>
    </div>
  );
}
