import React from "react";
import ReactDOM from "react-dom/client";
import { WebviewWindow, getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App";
import { GuardOverlay } from "./components/GuardOverlay";
import "./styles.css";

const params = new URLSearchParams(window.location.search);
const isGuard = params.get("guard") === "1";

if (!isGuard) {
  const current = getCurrentWebviewWindow();
  let guardWindow: WebviewWindow | null = null;
  window.addEventListener("nocturne:open-guard", () => {
    if (guardWindow) {
      guardWindow.show().catch(() => undefined);
      guardWindow.setFocus().catch(() => undefined);
      return;
    }
    guardWindow = new WebviewWindow("guard", {
      url: "/?guard=1",
      title: "Nocturne Guard",
      width: 1280,
      height: 720,
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      skipTaskbar: true,
      fullscreen: true,
      resizable: false,
      focus: true
    });
    guardWindow.once("tauri://destroyed", () => {
      guardWindow = null;
    });
  });
  current.show().catch(() => undefined);
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isGuard ? <GuardOverlay /> : <App />}</React.StrictMode>
);
