use std::{
    fs,
    path::Path,
    sync::Arc,
};

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    models::{OverlayTarget, ProcessEntry, ProtectedAppCandidate, ProtectedTarget, SecuritySettings},
    state::RuntimeState,
};

pub fn detect_popular_candidates(processes: &[ProcessEntry]) -> Vec<ProtectedAppCandidate> {
    let popular = [
        ("discord.exe", "Discord"),
        ("chrome.exe", "Google Chrome"),
        ("msedge.exe", "Microsoft Edge"),
        ("firefox.exe", "Mozilla Firefox"),
        ("brave.exe", "Brave"),
        ("opera.exe", "Opera"),
        ("telegram.exe", "Telegram"),
        ("steam.exe", "Steam"),
        ("code.exe", "Visual Studio Code"),
        ("obs64.exe", "OBS Studio"),
    ];

    popular
        .iter()
        .map(|(process_name, display_name)| ProtectedAppCandidate {
            process_name: (*process_name).to_string(),
            display_name: (*display_name).to_string(),
            installed: executable_exists(process_name),
            running: processes.iter().any(|p| p.name.eq_ignore_ascii_case(process_name)),
        })
        .collect()
}

fn executable_exists(name: &str) -> bool {
    let guesses = [
        format!(r"C:\Program Files\{}", name),
        format!(r"C:\Program Files (x86)\{}", name),
        format!(r"C:\Users\{}\AppData\Local\Programs\Discord\{}", std::env::var("USERNAME").unwrap_or_default(), name),
        format!(r"C:\Users\{}\AppData\Local\Microsoft\Edge\Application\msedge.exe", std::env::var("USERNAME").unwrap_or_default()),
        format!(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
        format!(r"C:\Program Files\Mozilla Firefox\firefox.exe"),
    ];

    guesses.iter().any(|path| Path::new(path).exists())
}

pub fn set_password(state: &Arc<RuntimeState>, password: &str, encrypt_files: bool) -> Result<SecuritySettings> {
    let hash = if password.trim().is_empty() {
        let inner = state.inner.lock();
        inner.password_hash.clone()
    } else {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)?
            .to_string();
        Some(hash)
    };

    {
        let mut inner = state.inner.lock();
        inner.password_hash = hash;
        inner.security.password_enabled = inner.password_hash.is_some();
        inner.security.encrypt_files = encrypt_files;
    }

    if encrypt_files {
        let payload = json!({
            "protected_targets": state.inner.lock().security.protected_targets,
            "hint": "placeholder for future encrypted token / password vault"
        });
        let _ = fs::write(&state.secure_vault_path, payload.to_string());
    }

    state.persist();
    Ok(get_settings(state))
}

pub fn verify_password(state: &Arc<RuntimeState>, password: &str) -> bool {
    let hash = {
        let inner = state.inner.lock();
        inner.password_hash.clone()
    };

    let Some(hash) = hash else {
        return true;
    };

    let parsed = PasswordHash::new(&hash);
    match parsed {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn get_settings(state: &Arc<RuntimeState>) -> SecuritySettings {
    let mut inner = state.inner.lock();
    inner.security.password_enabled = inner.password_hash.is_some();
    inner.security.clone()
}

pub fn set_targets(state: &Arc<RuntimeState>, targets: Vec<ProtectedTarget>) -> SecuritySettings {
    {
        let mut inner = state.inner.lock();
        inner.security.protected_targets = targets
            .into_iter()
            .filter(|t| !t.process_name.trim().is_empty())
            .collect();
    }
    state.persist();
    get_settings(state)
}

pub fn unlock_overlay(app: &AppHandle, state: &Arc<RuntimeState>) {
    {
        let mut inner = state.inner.lock();
        if let Some(OverlayTarget { pid, .. }) = inner.overlay_target.clone() {
            inner.locked_pids.remove(&pid);
        }
        inner.overlay_target = None;
        inner.security.overlay_active = false;
        inner.security.overlay_target = None;
    }

    if let Some(window) = app.get_webview_window("overlay") {
        let _ = app.emit("security://overlay_state", serde_json::json!(null));
        let _ = window.hide();
    }
}
