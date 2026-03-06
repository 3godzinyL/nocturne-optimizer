import { invoke } from "@tauri-apps/api/core";
import {
  AppSettings,
  AutostartEntry,
  OfflineProfile,
  OfflineProfileResult,
  OptimizationRule,
  OverviewBundle,
  RegistryCheck,
  SecuritySettings
} from "../types";

export const api = {
  getOverviewBundle: () => invoke<OverviewBundle>("get_overview_bundle"),
  getProcesses: () => invoke("get_processes"),
  setRules: (rules: OptimizationRule[]) =>
    invoke<OptimizationRule[]>("set_rules", { rules }),
  refreshAutostart: () => invoke<AutostartEntry[]>("refresh_autostart"),
  toggleAutostart: (entryId: string, enabled: boolean) =>
    invoke<AutostartEntry[]>("toggle_autostart", { entryId, enabled }),
  getOfflineProfiles: () => invoke<OfflineProfile[]>("get_offline_profiles"),
  runOfflineProfile: (profileId: string) =>
    invoke<OfflineProfileResult>("run_offline_profile", { profileId }),
  getRegistryChecks: () => invoke<RegistryCheck[]>("get_registry_checks"),
  fixRegistryCheck: (checkId: string) =>
    invoke<RegistryCheck[]>("fix_registry_check", { checkId }),
  getSecuritySettings: () => invoke<SecuritySettings>("get_security_settings"),
  setSecurityPassword: (password: string, encryptFiles: boolean) =>
    invoke<SecuritySettings>("set_security_password", { password, encryptFiles }),
  setProtectedTargets: (targets: { processName: string; displayName: string; enabled: boolean }[]) =>
    invoke<SecuritySettings>("set_protected_targets", { targets }),
  verifyAndUnlock: (password: string) =>
    invoke<boolean>("verify_and_unlock", { password }),
  updateSettings: (settings: AppSettings) =>
    invoke<AppSettings>("update_settings", { settings })
};
