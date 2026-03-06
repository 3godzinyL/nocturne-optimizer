use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub cpu_usage: f32,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub memory_used_pct: f32,
    pub swap_used_gb: f64,
    pub swap_total_gb: f64,
    pub uptime_secs: u64,
    pub refreshed_at: String,
    pub network_rx_mb: f64,
    pub network_tx_mb: f64,
    pub active_process_name: String,
    pub top_processes: Vec<ProcessEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub exe: String,
    pub cpu_usage: f32,
    pub memory_mb: f64,
    pub status: String,
    pub can_optimize: bool,
    pub is_foreground: bool,
    pub is_window_visible: bool,
    pub is_minimized: bool,
    pub rule_applied: Option<String>,
    pub optimization_level: Option<String>,
    pub window_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationRule {
    pub id: String,
    pub process_match: String,
    pub level: OptimizationLevel,
    pub only_when_not_foreground: bool,
    pub only_when_hidden: bool,
    pub trim_memory: bool,
    pub lower_priority: bool,
    pub suspend_on_hide: bool,
    pub enabled: bool,
}

impl Default for OptimizationRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            process_match: String::new(),
            level: OptimizationLevel::Balanced,
            only_when_not_foreground: true,
            only_when_hidden: false,
            trim_memory: true,
            lower_priority: true,
            suspend_on_hide: false,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Eco,
    Balanced,
    Freeze,
}

impl ToString for OptimizationLevel {
    fn to_string(&self) -> String {
        match self {
            OptimizationLevel::Eco => "Eco",
            OptimizationLevel::Balanced => "Balanced",
            OptimizationLevel::Freeze => "Freeze",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutostartEntry {
    pub id: String,
    pub source: String,
    pub name: String,
    pub command: String,
    pub location: String,
    pub enabled: bool,
    pub can_toggle: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineProfile {
    pub id: String,
    pub title: String,
    pub description: String,
    pub actions: Vec<String>,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OfflineProfileResult {
    pub profile_id: String,
    pub success: bool,
    pub output: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCheck {
    pub id: String,
    pub label: String,
    pub path: String,
    pub value_name: String,
    pub current_value: String,
    pub recommended_value: String,
    pub status: String,
    pub severity: String,
    pub description: String,
    pub can_fix: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedTarget {
    pub process_name: String,
    pub display_name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedAppCandidate {
    pub process_name: String,
    pub display_name: String,
    pub installed: bool,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTarget {
    pub pid: u32,
    pub process_name: String,
    pub display_name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySettings {
    pub password_enabled: bool,
    pub encrypt_files: bool,
    pub protected_targets: Vec<ProtectedTarget>,
    pub popular_candidates: Vec<ProtectedAppCandidate>,
    pub overlay_active: bool,
    pub overlay_target: Option<OverlayTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub refresh_interval_ms: u64,
    pub aggressive_mode: bool,
    pub auto_apply_rules: bool,
    pub minimize_to_tray: bool,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 2500,
            aggressive_mode: false,
            auto_apply_rules: true,
            minimize_to_tray: true,
            theme: "nocturne".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverviewBundle {
    pub dashboard: DashboardSnapshot,
    pub processes: Vec<ProcessEntry>,
    pub rules: Vec<OptimizationRule>,
    pub autostart: Vec<AutostartEntry>,
    pub offline_profiles: Vec<OfflineProfile>,
    pub registry_checks: Vec<RegistryCheck>,
    pub security: SecuritySettings,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedConfig {
    pub rules: Vec<OptimizationRule>,
    pub settings: AppSettings,
    pub security: PersistedSecurity,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSecurity {
    pub password_hash: Option<String>,
    pub encrypt_files: bool,
    pub protected_targets: Vec<ProtectedTarget>,
}
