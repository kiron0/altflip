import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

const tauri = "__TAURI_INTERNALS__" in window;
const ua = navigator.userAgent;
document.documentElement.dataset.os = !tauri ? "web" : /Mac/.test(ua) ? "macos" : /Win/.test(ua) ? "windows" : "linux";

const surface = new URLSearchParams(window.location.search).get("surface");
document.documentElement.dataset.surface = surface === "quick" ? "quick" : "main";
const Root = surface === "quick" ? (await import("./quick/QuickPanel")).QuickPanel : (await import("./App")).App;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
