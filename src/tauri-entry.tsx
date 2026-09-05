// Desktop (Tauri) SPA entry point.
//
// The Lovable web pipeline renders App through its TanStack Start runtime,
// which never emits a static index.html. For the packaged desktop app we
// mount the exact same App imperatively so Tauri can embed a plain SPA.
import { createRoot } from "react-dom/client";
import App from "./main";

createRoot(document.getElementById("root")!).render(<App />);
