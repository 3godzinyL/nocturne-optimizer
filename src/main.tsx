import React from "react";
import ReactDOM from "react-dom/client";
import { WebviewWindow, getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App";
import { GuardOverlay } from "./components/GuardOverlay";
import { DesktopHud } from "./components/DesktopHud";
import { api } from "./lib/tauri";
import "./styles.css";

const params = new URLSearchParams(window.location.search);
const currentWindow = getCurrentWebviewWindow();
const currentLabel = currentWindow.label;
const isGuard = params.get("guard") === "1" || currentLabel === "guard";
const isHud = params.get("hud") === "1" || currentLabel === "hud";

function buildAppUrl(extraQuery: string) {
  const url = new URL(window.location.href);
  url.search = extraQuery;
  return url.toString();
}

if (!isGuard && !isHud) {
  let guardWindow: WebviewWindow | null = null;
  window.addEventListener("nocturne:open-guard", async () => {
    const runtime = await api.getSecurityRuntime().catch(() => null);
    const bounds = runtime?.overlayBounds;

    if (guardWindow) {
      await guardWindow.close().catch(() => undefined);
      guardWindow = null;
    }

    guardWindow = new WebviewWindow("guard", {
      url: buildAppUrl("?guard=1"),
      title: "Nocturne Guard",
      width: bounds?.width && bounds.width > 260 ? bounds.width : 1280,
      height: bounds?.height && bounds.height > 180 ? bounds.height : 720,
      x: bounds?.x,
      y: bounds?.y,
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      skipTaskbar: true,
      fullscreen: !bounds,
      resizable: false,
      focus: true
    });

    guardWindow.once("tauri://created", () => {
      guardWindow?.setFocus().catch(() => undefined);
      guardWindow?.show().catch(() => undefined);
    });

    guardWindow.once("tauri://destroyed", () => {
      guardWindow = null;
    });
  });

  currentWindow.show().catch(() => undefined);
}

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      {isGuard ? <GuardOverlay /> : isHud ? <DesktopHud /> : <App />}
    </React.StrictMode>
  );
}
