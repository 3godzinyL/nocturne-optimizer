import type { LucideIcon } from "lucide-react";
import {
  AppWindow,
  Chrome,
  Gamepad2,
  Globe,
  MessageCircleMore,
  Music4,
  PanelTop,
  Shield,
  TerminalSquare,
  Twitch,
  Video,
  FolderCog
} from "lucide-react";

const iconMap: Record<string, LucideIcon> = {
  chrome: Chrome,
  msedge: Globe,
  firefox: Globe,
  brave: Globe,
  opera: Globe,
  discord: MessageCircleMore,
  telegram: MessageCircleMore,
  steam: Gamepad2,
  spotify: Music4,
  code: TerminalSquare,
  powershell: TerminalSquare,
  cmd: TerminalSquare,
  explorer: FolderCog,
  teams: Video,
  obs: Video,
  twitch: Twitch,
  defender: Shield,
  securityhealthservice: Shield,
  windowssecurity: Shield,
  taskmgr: PanelTop
};

export function iconForProcess(name: string): LucideIcon {
  const clean = name.toLowerCase().replace(/\.exe$/i, "");
  return iconMap[clean] ?? AppWindow;
}

export function ProcessIcon({ name, className = "process-glyph" }: { name: string; className?: string }) {
  const Icon = iconForProcess(name);
  return <span className={className}><Icon size={16} /></span>;
}
