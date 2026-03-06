export type Mode = "Eco" | "Balanced" | "Freeze";

export interface ProcessInfo {
  pid: number;
  name: string;
  exe: string;
  cpu: number;
  memoryMb: number;
  status: string;
  foreground: boolean;
  optimizable: boolean;
  optimizedState: string;
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
}

export interface AutostartItem {
  id: string;
  source: string;
  name: string;
  path: string;
  itemType: string;
  enabled: boolean;
  details: string;
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
}

export interface SecurityConfig {
  passwordSet: boolean;
  fileProtection: boolean;
  protectedApps: string[];
  lockEnabled: boolean;
}

export interface SecurityRuntime {
  locked: boolean;
  lockedApp?: string | null;
  presentPopularApps: string[];
}

export interface SettingsState {
  refreshMs: number;
  autoApplyRules: boolean;
  aggressiveMode: boolean;
  minimizeToTray: boolean;
}

// Compatibility aliases for legacy frontend files
export type AppSettings = SettingsState;
export type AutostartEntry = AutostartItem;
export type OfflineProfile = { id: string; title: string; description: string; actions: string[]; risk: string };
export type OfflineProfileResult = OfflinePresetResult;
export type OverviewBundle = { dashboard?: unknown; processes?: ProcessInfo[]; rules?: OptimizationRule[]; autostart?: AutostartItem[]; offlineProfiles?: OfflineProfile[]; registryChecks?: RegistryHealthItem[]; security?: SecurityConfig; settings?: SettingsState };
export type RegistryCheck = RegistryHealthItem;
export type SecuritySettings = SecurityConfig;
