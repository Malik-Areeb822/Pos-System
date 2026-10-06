import { invoke } from "@tauri-apps/api/core";
import type { LockStatus } from "@/lib/license-store";

export async function checkSystemLock(): Promise<LockStatus> {
  return invoke<LockStatus>("check_system_lock");
}

export async function unlockWithPassword(password: string): Promise<LockStatus> {
  return invoke<LockStatus>("unlock_with_password", { input: { password } });
}
