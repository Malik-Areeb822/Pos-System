import { createFileRoute, redirect } from "@tanstack/react-router";

// Legacy path — the POS login now lives at "/".
export const Route = createFileRoute("/auth")({
  beforeLoad: () => {
    throw redirect({ to: "/" });
  },
});
