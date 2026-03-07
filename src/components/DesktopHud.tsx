import { CSSProperties, useEffect, useMemo, useRef, useState } from "react";
import { Activity, Clock3, HardDrive, Layers3, Sparkles } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "../lib/tauri";
import type { SettingsState, SystemSnapshot } from "../types";

const defaultSettings: SettingsState = {
  refreshMs: 3800,
  autoApplyRules: true,
  aggressiveMode: false,
  minimizeToTray: true,
  hudEnabled: false,
  hudHotkey: "Ctrl+Shift+H",
  hudCorner: "top-right",
  hudOpacity: 82,
  hudScale: 100,
  hudShowCpu: true,
  hudShowRam: true,
  hudShowProcesses: true,
  hudShowUptime: true,
  hudShowTopApp: true,
  hudPositionMode: "corner",
  hudX: 32,
  hudY: 32,
  hudWidth: 420,
  hudHeight: 220,
  launchOnLogin: false
};

function fmtUptime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${mins}m`;
}

export function DesktopHud() {
  const [snapshot, setSnapshot] = useState<SystemSnapshot | null>(null);
  const [settings, setSettings] = useState<SettingsState>(defaultSettings);
  const busyRef = useRef(false);
  const settingsBusyRef = useRef(false);

  const refreshSnapshot = async () => {
    if (busyRef.current) return;
    busyRef.current = true;
    try {
      const nextSnapshot = await api.getSystemSnapshot();
      setSnapshot(nextSnapshot);
    } finally {
      busyRef.current = false;
    }
  };

  const refreshSettings = async () => {
    if (settingsBusyRef.current) return;
    settingsBusyRef.current = true;
    try {
      const nextSettings = await api.getSettings();
      setSettings((current) => ({ ...current, ...nextSettings }));
    } finally {
      settingsBusyRef.current = false;
    }
  };

  useEffect(() => {
    const win = getCurrentWindow();
    let cleanup: (() => void | Promise<void>) = () => undefined;
    document.documentElement.classList.add("hud-window");
    document.body.classList.add("hud-window");
    const root = document.getElementById("root");
    root?.classList.add("hud-window-root");
    win.setAlwaysOnTop(true).catch(() => undefined);
    win.setSkipTaskbar(true).catch(() => undefined);
    win.setFocusable(false).catch(() => undefined);
    win.setIgnoreCursorEvents(true).catch(() => undefined);
    win.setResizable(false).catch(() => undefined);
    listen("nocturne://hud-sync", () => {
      refreshSettings().catch(() => undefined);
      refreshSnapshot().catch(() => undefined);
    }).then((unlisten) => {
      cleanup = unlisten;
    }).catch(() => undefined);
    refreshSettings().catch(() => undefined);
    refreshSnapshot().catch(() => undefined);
    const timer = window.setInterval(() => refreshSnapshot().catch(() => undefined), Math.max(3200, settings.refreshMs || 3200));
    return () => {
      Promise.resolve(cleanup()).catch(() => undefined);
      document.documentElement.classList.remove("hud-window");
      document.body.classList.remove("hud-window");
      root?.classList.remove("hud-window-root");
      window.clearInterval(timer);
    };
  }, [settings.refreshMs]);

  const topProcess = useMemo(() => {
    if (!snapshot?.processes?.length) return null;
    return [...snapshot.processes].sort((a, b) => b.cpu - a.cpu)[0];
  }, [snapshot]);

  const panelStyle = useMemo(() => {
    const scale = settings.hudScale / 100;
    return {
      opacity: settings.hudOpacity / 100,
      transform: `scale(${scale})`,
      transformOrigin: "top left",
      width: `${settings.hudWidth}px`,
      minHeight: `${settings.hudHeight}px`
    } as CSSProperties;
  }, [settings]);

  if (!settings.hudEnabled || !snapshot) return <div className="desktop-hud-empty" />;

  return (
    <div className="desktop-hud-root">
      <div className="desktop-hud-shell" style={panelStyle}>
        <div className="desktop-hud-header">
          <div>
            <div className="eyebrow">global translucent hud</div>
            <div className="desktop-hud-title">Nocturne Pulse</div>
          </div>
          <div className="desktop-hud-sub">
            <Sparkles size={13} /> {settings.hudHotkey} · toggle HUD
          </div>
        </div>
        <div className="desktop-hud-grid">
          {settings.hudShowCpu ? <div className="desktop-hud-chip"><Activity size={14} /> CPU <strong>{snapshot.cpuUsage.toFixed(1)}%</strong></div> : null}
          {settings.hudShowRam ? <div className="desktop-hud-chip"><HardDrive size={14} /> RAM <strong>{snapshot.ramUsedGb.toFixed(1)} / {snapshot.ramTotalGb.toFixed(1)} GB</strong></div> : null}
          {settings.hudShowProcesses ? <div className="desktop-hud-chip"><Layers3 size={14} /> PROCESY <strong>{snapshot.processes.length}</strong></div> : null}
          {settings.hudShowUptime ? <div className="desktop-hud-chip"><Clock3 size={14} /> UPTIME <strong>{fmtUptime(snapshot.uptimeSeconds)}</strong></div> : null}
        </div>
        {settings.hudShowTopApp && topProcess ? (
          <div className="desktop-hud-focus">
            <div className="eyebrow">top load</div>
            <div className="desktop-hud-focus-row">
              <span>{topProcess.displayName}</span>
              <strong>{topProcess.cpu.toFixed(1)}% CPU · {topProcess.memoryMb.toFixed(0)} MB</strong>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
