import { Activity, HardDrive, Layers3 } from "lucide-react";
import type { SettingsState, SystemSnapshot } from "../types";

const cornerClass: Record<SettingsState["hudCorner"], string> = {
  "top-left": "hud-top-left",
  "top-right": "hud-top-right",
  "bottom-left": "hud-bottom-left",
  "bottom-right": "hud-bottom-right"
};

export function HudPanel({ snapshot, settings, visible }: { snapshot: SystemSnapshot | null; settings: SettingsState; visible: boolean }) {
  if (!visible || !settings.hudEnabled || !snapshot) return null;

  return (
    <div
      className={`hud-panel ${cornerClass[settings.hudCorner]}`}
      style={{ opacity: settings.hudOpacity / 100, transform: `scale(${settings.hudScale / 100})` }}
    >
      <div className="hud-chip"><Activity size={14} /> CPU {snapshot.cpuUsage.toFixed(1)}%</div>
      <div className="hud-chip"><HardDrive size={14} /> RAM {snapshot.ramUsedGb.toFixed(1)} / {snapshot.ramTotalGb.toFixed(1)} GB</div>
      <div className="hud-chip"><Layers3 size={14} /> {snapshot.processes.length} proc.</div>
    </div>
  );
}
