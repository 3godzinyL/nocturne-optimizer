import { invoke } from "@tauri-apps/api/core";
import type {
  AutostartItem,
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
  getSecurityConfig: () => invoke<SecurityConfig>("get_security_config"),
  saveSecurityConfig: (payload: { password?: string; fileProtection: boolean; protectedApps: string[]; lockEnabled: boolean }) =>
    invoke<void>("save_security_config", { payload }),
  getSecurityRuntime: () => invoke<SecurityRuntime>("get_security_runtime"),
  unlockGuard: (password: string) => invoke<boolean>("unlock_guard", { password }),
  getSettings: () => invoke<SettingsState>("get_settings"),
  saveSettings: (settings: SettingsState) => invoke<void>("save_settings", { settings })
};
