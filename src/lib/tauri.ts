import { invoke } from "@tauri-apps/api/core";
import type {
  AppInventory,
  AutostartItem,
  NetworkOverview,
  NetworkRule,
  NetworkTuneResult,
  OfflinePresetResult,
  OptimizationRule,
  RegistryHealthItem,
  SecurityConfig,
  SecurityRuntime,
  SettingsState,
  SystemSnapshot
} from "../types";

export const api = {
  getSystemSnapshot: () => invoke<SystemSnapshot>("get_system_snapshot"),
  getRules: () => invoke<OptimizationRule[]>("get_rules"),
  saveRules: (rules: OptimizationRule[]) => invoke<void>("save_rules", { rules }),
  listAutostartItems: () => invoke<AutostartItem[]>("list_autostart_items"),
  toggleAutostartItem: (item: AutostartItem, enable: boolean) =>
    invoke<void>("toggle_autostart_item", { item, enable }),
  runOfflinePreset: (presetId: string) => invoke<OfflinePresetResult>("run_offline_preset", { presetId }),
  getRegistryHealth: () => invoke<RegistryHealthItem[]>("get_registry_health"),
  runRegistryAuditConsole: (mode: "scan" | "repair") => invoke<void>("run_registry_audit_console", { mode }),
  getSecurityConfig: () => invoke<SecurityConfig>("get_security_config"),
  saveSecurityConfig: (payload: {
    password?: string;
    fileProtection: boolean;
    protectedApps: string[];
    lockEnabled: boolean;
    lockOnRestore: boolean;
    lockOnActivate: boolean;
    graceMinutes: number;
    appPasswordOnStart: boolean;
  }) => invoke<void>("save_security_config", { payload }),
  getSecurityRuntime: () => invoke<SecurityRuntime>("get_security_runtime"),
  unlockGuard: (password: string) => invoke<boolean>("unlock_guard", { password }),
  verifyPassword: (password: string) => invoke<boolean>("verify_password", { password }),
  getSettings: () => invoke<SettingsState>("get_settings"),
  saveSettings: (settings: SettingsState) => invoke<void>("save_settings", { settings }),
  toggleHudWindow: () => invoke<void>("toggle_hud_window"),
  getAppInventory: () => invoke<AppInventory>("get_app_inventory"),
  getSelfAutostart: () => invoke<boolean>("get_self_autostart"),
  setSelfAutostart: (enable: boolean) => invoke<void>("set_self_autostart", { enable }),
  getNetworkOverview: () => invoke<NetworkOverview>("get_network_overview"),
  saveNetworkRules: (rules: NetworkRule[]) => invoke<void>("save_network_rules", { rules }),
  runNetworkTune: () => invoke<NetworkTuneResult>("run_network_tune")
};
