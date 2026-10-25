import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { startThemeSync } from "./lib/theme";
import "./styles.css";

startThemeSync();

// The Playwright smoke suite runs the dev server in `e2e` mode and needs the Tauri IPC mocked before the first
// render. Vite replaces MODE statically, so a production build drops this branch and the mock chunk.
const boot =
  import.meta.env.MODE === "e2e"
    ? import("../e2e/fixtures/mockTauri").then((m) => m.installTauriMocks())
    : Promise.resolve();

void boot.then(() =>
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  ),
);
