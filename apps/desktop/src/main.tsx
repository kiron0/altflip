import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

const tauri = "__TAURI_INTERNALS__" in window;
if (import.meta.env.DEV && !tauri) {
  const { installMocks } = await import("./dev/mock");
  installMocks();
}

const surface = new URLSearchParams(window.location.search).get("surface");
const Root = surface === "quick" ? (await import("./quick/QuickPanel")).QuickPanel : (await import("./App")).App;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
