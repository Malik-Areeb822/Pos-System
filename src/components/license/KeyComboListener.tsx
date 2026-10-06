import { useEffect, useRef } from "react";
import { useLicenseStore } from "@/lib/license-store";
import { useQueryClient } from "@tanstack/react-query";

const SECRET_COMBO = [
  "l",
  "o",
  "c",
  "k",
  "d",
  "o",
  "w",
  "n",
  "s",
  "y",
  "s",
  "t",
  "e",
  "m",
] as const;
const COMBO_TIMEOUT_MS = 4000;

export function KeyComboListener() {
  const setStatus = useLicenseStore((s) => s.setStatus);
  const queryClient = useQueryClient();
  const pressed = useRef<string[]>([]);
  const lastKeyTime = useRef<number>(0);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      // Ignore if an input/textarea/contenteditable is focused
      const el = document.activeElement;
      if (
        el &&
        (el.tagName === "INPUT" ||
          el.tagName === "TEXTAREA" ||
          (el as HTMLElement).isContentEditable)
      ) {
        return;
      }

      const now = Date.now();
      const key = e.key.toLowerCase();

      // Reset if timeout exceeded or non-alpha key pressed
      if (now - lastKeyTime.current > COMBO_TIMEOUT_MS || !/^[a-z]$/.test(key)) {
        pressed.current = [];
      }
      lastKeyTime.current = now;

      pressed.current.push(key);

      // Keep only last N keys (length of secret combo)
      if (pressed.current.length > SECRET_COMBO.length) {
        pressed.current = pressed.current.slice(-SECRET_COMBO.length);
      }

      // Check match
      if (
        pressed.current.length === SECRET_COMBO.length &&
        pressed.current.every((k, i) => k === SECRET_COMBO[i])
      ) {
        pressed.current = [];
        import("@tauri-apps/api/core").then(({ invoke }) => {
          invoke("set_system_lock", { input: { locked: true } })
            .then(() => {
              setStatus({ is_locked: true });
              queryClient.clear();
            })
            .catch(() => {});
        });
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [setStatus, queryClient]);

  return null;
}
