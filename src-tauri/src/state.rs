use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::Arc,
};

use directories::ProjectDirs;
use parking_lot::Mutex;

use crate::models::{
    AppSettings, DashboardSnapshot, OptimizationLevel, OptimizationRule, OverlayTarget,
    PersistedConfig, PersistedSecurity, ProcessEntry, SecuritySettings,
};

#[derive(Clone)]
pub struct AppliedOptimization {
    pub rule_id: String,
    pub level: OptimizationLevel,
}

#[derive(Default)]
pub struct InnerState {
    pub dashboard: DashboardSnapshot,
    pub processes: Vec<ProcessEntry>,
    pub rules: Vec<OptimizationRule>,
    pub settings: AppSettings,
    pub security: SecuritySettings,
    pub password_hash: Option<String>,
    pub optimized: HashMap<u32, AppliedOptimization>,
    pub locked_pids: HashSet<u32>,
    pub last_minimized: HashMap<u32, bool>,
    pub overlay_target: Option<OverlayTarget>,
}

pub struct RuntimeState {
    pub inner: Mutex<InnerState>,
    pub config_path: PathBuf,
    pub secure_vault_path: PathBuf,
}

impl RuntimeState {
    pub fn load() -> Arc<Self> {
        let project_dirs = ProjectDirs::from("com", "GROUP", "NocturneOptimizer")
            .expect("cannot create app data path");
        let config_dir = project_dirs.config_dir();
        let _ = fs::create_dir_all(config_dir);

        let config_path = config_dir.join("config.json");
        let secure_vault_path = config_dir.join("vault.sealed");

        let persisted = fs::read_to_string(&config_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<PersistedConfig>(&raw).ok())
            .unwrap_or_default();

        let security = SecuritySettings {
            password_enabled: persisted.security.password_hash.is_some(),
            encrypt_files: persisted.security.encrypt_files,
            protected_targets: persisted.security.protected_targets.clone(),
            popular_candidates: vec![],
            overlay_active: false,
            overlay_target: None,
        };

        Arc::new(Self {
            inner: Mutex::new(InnerState {
                dashboard: DashboardSnapshot::default(),
                processes: vec![],
                rules: persisted.rules,
                settings: persisted.settings,
                security,
                password_hash: persisted.security.password_hash,
                optimized: HashMap::new(),
                locked_pids: HashSet::new(),
                last_minimized: HashMap::new(),
                overlay_target: None,
            }),
            config_path,
            secure_vault_path,
        })
    }

    pub fn persist(&self) {
        let inner = self.inner.lock();
        let payload = PersistedConfig {
            rules: inner.rules.clone(),
            settings: inner.settings.clone(),
            security: PersistedSecurity {
                password_hash: inner.password_hash.clone(),
                encrypt_files: inner.security.encrypt_files,
                protected_targets: inner.security.protected_targets.clone(),
            },
        };

        if let Ok(raw) = serde_json::to_string_pretty(&payload) {
            let _ = fs::write(&self.config_path, raw);
        }
    }

    pub fn update_security_flags(&self, encrypt_files: bool) {
        let mut inner = self.inner.lock();
        inner.security.encrypt_files = encrypt_files;
    }
}
