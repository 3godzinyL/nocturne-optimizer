export type Mode = "Eco" | "Balanced" | "Freeze";
export type HudCorner = "top-left" | "top-right" | "bottom-left" | "bottom-right";
export type HudPositionMode = "corner" | "custom";

export interface ProcessInfo {
  pid: number;
  name: string;
  displayName: string;
  exe: string;
  cpu: number;
  memoryMb: number;
  status: string;
  foreground: boolean;
  optimizable: boolean;
  optimizedState: string;
  iconHint: string;
  ruleMatched?: string | null;
}

export interface SystemSnapshot {
  cpuUsage: number;
  ramUsedGb: number;
  ramTotalGb: number;
  swapUsedGb: number;
  swapTotalGb: number;
  uptimeSeconds: number;
  foregroundPid?: number | null;
  processes: ProcessInfo[];
}

export interface OptimizationRule {
  id: string;
  processName: string;
  mode: Mode;
  requireBackground: boolean;
  autoResume: boolean;
  enabled: boolean;
  cpuLimitPct?: number;
  ramLimitPct?: number;
  diskLimitPct?: number;
  gpuLimitPct?: number;
  familyKey?: string;
}

export interface AutostartItem {
  id: string;
  source: string;
  name: string;
  path: string;
  itemType: string;
  enabled: boolean;
  details: string;
  iconHint: string;
}

export interface OfflinePresetResult {
  preset: string;
  success: boolean;
  details: string;
}

export interface RegistryHealthItem {
  keyPath: string;
  valueName: string;
  current: string;
  recommended: string;
  healthy: boolean;
  meaning: string;
  severity: string;
}

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SecurityConfig {
  passwordSet: boolean;
  fileProtection: boolean;
  protectedApps: string[];
  lockEnabled: boolean;
  lockOnRestore: boolean;
  lockOnActivate: boolean;
  graceMinutes: number;
  appPasswordOnStart: boolean;
}

export interface SecurityRuntime {
  locked: boolean;
  lockedApp?: string | null;
  presentPopularApps: string[];
  overlayBounds?: WindowBounds | null;
}

export interface SettingsState {
  refreshMs: number;
  autoApplyRules: boolean;
  aggressiveMode: boolean;
  minimizeToTray: boolean;
  hudEnabled: boolean;
  hudHotkey: string;
  hudCorner: HudCorner;
  hudOpacity: number;
  hudScale: number;
  hudShowCpu: boolean;
  hudShowRam: boolean;
  hudShowProcesses: boolean;
  hudShowUptime: boolean;
  hudShowTopApp: boolean;
  hudPositionMode: HudPositionMode;
  hudX: number;
  hudY: number;
  hudWidth: number;
  hudHeight: number;
  launchOnLogin: boolean;
}

export interface WindowsBundleApp {
  id: string;
  name: string;
  publisher: string;
  path: string;
  installed: boolean;
  removable: boolean;
  status: string;
  startupEnabled: boolean;
  permissionsSummary: string;
  iconHint: string;
}

export interface InstalledProgram {
  id: string;
  name: string;
  publisher: string;
  version: string;
  path: string;
  startupEnabled: boolean;
  kind: string;
  permissionsSummary: string;
  iconHint: string;
}

export interface AppInventory {
  windowsApps: WindowsBundleApp[];
  installedPrograms: InstalledProgram[];
}

export interface NetworkAdapter {
  name: string;
  status: string;
  linkSpeed: string;
  macAddress: string;
  ipv4: string;
  sentMb: number;
  receivedMb: number;
}

export interface NetworkRule {
  id: string;
  processName: string;
  limitKbps: number;
  enabled: boolean;
  note: string;
}

export interface NetworkOverview {
  adapters: NetworkAdapter[];
  rules: NetworkRule[];
}

export interface NetworkTuneResult {
  success: boolean;
  summary: string;
}

// Compatibility aliases for older files
export type AppSettings = SettingsState;
export type AutostartEntry = AutostartItem;
export type OfflineProfile = {
  id: string;
  title: string;
  description: string;
  actions: string[];
  risk: string;
};
export type OfflineProfileResult = OfflinePresetResult;
export type OverviewBundle = {
  dashboard?: unknown;
  processes?: ProcessInfo[];
  rules?: OptimizationRule[];
  autostart?: AutostartItem[];
  registryChecks?: RegistryHealthItem[];
  security?: SecurityConfig;
  settings?: SettingsState;
};
export type RegistryCheck = RegistryHealthItem;
export type SecuritySettings = SecurityConfig;
