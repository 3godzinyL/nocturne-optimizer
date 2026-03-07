#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System, UpdateKind};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND, RECT},
    System::{
        ProcessStatus::K32EmptyWorkingSet,
        Threading::{
            OpenProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS,
            NORMAL_PRIORITY_CLASS, PROCESS_QUERY_INFORMATION, PROCESS_SET_INFORMATION,
            PROCESS_SET_QUOTA,
        },
    },
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProcessInfo {
    pid: i64,
    name: String,
    display_name: String,
    exe: String,
    cpu: f32,
    memory_mb: f64,
    status: String,
    foreground: bool,
    optimizable: bool,
    optimized_state: String,
    icon_hint: String,
    rule_matched: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct SystemSnapshot {
    cpu_usage: f32,
    ram_used_gb: f64,
    ram_total_gb: f64,
    swap_used_gb: f64,
    swap_total_gb: f64,
    uptime_seconds: u64,
    foreground_pid: Option<i64>,
    processes: Vec<ProcessInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
struct OptimizationRule {
    id: String,
    process_name: String,
    mode: String,
    require_background: bool,
    auto_resume: bool,
    enabled: bool,
    cpu_limit_pct: u8,
    ram_limit_pct: u8,
    disk_limit_pct: u8,
    gpu_limit_pct: u8,
    family_key: String,
}

impl Default for OptimizationRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            process_name: String::new(),
            mode: "Balanced".into(),
            require_background: true,
            auto_resume: true,
            enabled: true,
            cpu_limit_pct: 65,
            ram_limit_pct: 70,
            disk_limit_pct: 60,
            gpu_limit_pct: 55,
            family_key: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AutostartItem {
    id: String,
    source: String,
    name: String,
    path: String,
    item_type: String,
    enabled: bool,
    details: String,
    icon_hint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OfflinePresetResult {
    preset: String,
    success: bool,
    details: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RegistryHealthItem {
    key_path: String,
    value_name: String,
    current: String,
    recommended: String,
    healthy: bool,
    meaning: String,
    severity: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct WindowBounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecurityConfig {
    password_hash: Option<String>,
    file_protection: bool,
    protected_apps: Vec<String>,
    lock_enabled: bool,
    lock_on_restore: bool,
    lock_on_activate: bool,
    grace_minutes: u32,
    app_password_on_start: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecurityConfigView {
    password_set: bool,
    file_protection: bool,
    protected_apps: Vec<String>,
    lock_enabled: bool,
    lock_on_restore: bool,
    lock_on_activate: bool,
    grace_minutes: u32,
    app_password_on_start: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            password_hash: None,
            file_protection: false,
            protected_apps: vec![],
            lock_enabled: false,
            lock_on_restore: true,
            lock_on_activate: true,
            grace_minutes: 0,
            app_password_on_start: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecurityConfigPayload {
    password: Option<String>,
    file_protection: bool,
    protected_apps: Vec<String>,
    lock_enabled: bool,
    lock_on_restore: bool,
    lock_on_activate: bool,
    grace_minutes: u32,
    app_password_on_start: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct SecurityRuntime {
    locked: bool,
    locked_app: Option<String>,
    present_popular_apps: Vec<String>,
    overlay_bounds: Option<WindowBounds>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
struct SettingsState {
    refresh_ms: u64,
    auto_apply_rules: bool,
    aggressive_mode: bool,
    minimize_to_tray: bool,
    hud_enabled: bool,
    hud_hotkey: String,
    hud_corner: String,
    hud_opacity: u8,
    hud_scale: u8,
    hud_show_cpu: bool,
    hud_show_ram: bool,
    hud_show_processes: bool,
    hud_show_uptime: bool,
    hud_show_top_app: bool,
    hud_position_mode: String,
    hud_x: i32,
    hud_y: i32,
    hud_width: i32,
    hud_height: i32,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            refresh_ms: 3800,
            auto_apply_rules: true,
            aggressive_mode: false,
            minimize_to_tray: true,
            hud_enabled: false,
            hud_hotkey: "Ctrl+Shift+H".into(),
            hud_corner: "top-right".into(),
            hud_opacity: 82,
            hud_scale: 100,
            hud_show_cpu: true,
            hud_show_ram: true,
            hud_show_processes: true,
            hud_show_uptime: true,
            hud_show_top_app: true,
            hud_position_mode: "corner".into(),
            hud_x: 32,
            hud_y: 32,
            hud_width: 420,
            hud_height: 220,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct WindowsBundleApp {
    id: String,
    name: String,
    publisher: String,
    path: String,
    installed: bool,
    removable: bool,
    status: String,
    startup_enabled: bool,
    permissions_summary: String,
    icon_hint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct InstalledProgram {
    id: String,
    name: String,
    publisher: String,
    version: String,
    path: String,
    startup_enabled: bool,
    kind: String,
    permissions_summary: String,
    icon_hint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct AppInventory {
    windows_apps: Vec<WindowsBundleApp>,
    installed_programs: Vec<InstalledProgram>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct NetworkAdapter {
    name: String,
    status: String,
    link_speed: String,
    mac_address: String,
    ipv4: String,
    sent_mb: f64,
    received_mb: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
struct NetworkRule {
    id: String,
    process_name: String,
    limit_kbps: u32,
    enabled: bool,
    note: String,
}

impl Default for NetworkRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            process_name: String::new(),
            limit_kbps: 4096,
            enabled: true,
            note: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct NetworkOverview {
    adapters: Vec<NetworkAdapter>,
    rules: Vec<NetworkRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct NetworkTuneResult {
    success: bool,
    summary: String,
}

struct SharedState {
    rules: Mutex<Vec<OptimizationRule>>,
    network_rules: Mutex<Vec<NetworkRule>>,
    optimized: Mutex<HashMap<i64, String>>,
    security_config: Mutex<SecurityConfig>,
    security_runtime: Mutex<SecurityRuntime>,
    settings: Mutex<SettingsState>,
    snapshot_cache: Mutex<SystemSnapshot>,
    armed_apps: Mutex<HashMap<String, Instant>>,
    last_foreground: Mutex<Option<String>>,
    hud_shortcut: Mutex<Option<String>>,
    system: Mutex<System>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            rules: Mutex::new(vec![]),
            network_rules: Mutex::new(vec![]),
            optimized: Mutex::new(HashMap::new()),
            security_config: Mutex::new(SecurityConfig::default()),
            security_runtime: Mutex::new(SecurityRuntime::default()),
            settings: Mutex::new(SettingsState::default()),
            snapshot_cache: Mutex::new(SystemSnapshot::default()),
            armed_apps: Mutex::new(HashMap::new()),
            last_foreground: Mutex::new(None),
            hud_shortcut: Mutex::new(None),
            system: Mutex::new(System::new_with_specifics(
                RefreshKind::new()
                    .with_memory(MemoryRefreshKind::everything())
                    .with_cpu(CpuRefreshKind::everything())
                    .with_processes(ProcessRefreshKind::new()
                        .with_cpu()
                        .with_memory()
                        .with_exe(UpdateKind::OnlyIfNotSet)),
            )),
        }
    }
}

fn state_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("NocturneOptimizer");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn rules_path() -> PathBuf { state_dir().join("rules.json") }
fn security_path() -> PathBuf { state_dir().join("security.json") }
fn settings_path() -> PathBuf { state_dir().join("settings.json") }
fn network_rules_path() -> PathBuf { state_dir().join("network-rules.json") }
fn vault_path() -> PathBuf { state_dir().join("vault.sealed") }

fn load_json<T: for<'a> Deserialize<'a> + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|txt| serde_json::from_str::<T>(&txt).ok())
        .unwrap_or_default()
}

fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let body = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, body).map_err(|e| e.to_string())
}

fn normalize_password(password: &str) -> String {
    password.trim().replace("
", "
")
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalize_password(password).as_bytes());
    B64.encode(hasher.finalize())
}

fn verify_password_value(stored: &Option<String>, candidate: &str) -> bool {
    let normalized = normalize_password(candidate);
    if normalized.is_empty() {
        return false;
    }
    match stored {
        Some(saved) => {
            saved == &normalized
                || saved == &hash_password(&normalized)
                || saved == &hash_password(candidate)
                || saved.trim() == normalized.trim()
        }
        None => false,
    }
}

fn powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn psq(input: &str) -> String {
    input.replace('"', "``\"")
}

fn icon_hint_for_name(name: &str) -> String {
    let lower = name.to_lowercase();
    [
        "discord", "chrome", "msedge", "firefox", "brave", "opera", "telegram", "steam",
        "explorer", "code", "powershell", "cmd", "spotify", "teams", "obs", "defender",
    ]
    .into_iter()
    .find(|probe| lower.contains(probe))
    .unwrap_or_else(|| lower.trim_end_matches(".exe"))
    .to_string()
}

fn pretty_name(name: &str) -> String {
    let clean = name.trim_end_matches(".exe").replace('_', " ");
    let mut out = String::new();
    let mut cap = true;
    for ch in clean.chars() {
        if cap {
            out.extend(ch.to_uppercase());
            cap = false;
        } else {
            out.push(ch);
        }
        if ch == ' ' || ch == '-' {
            cap = true;
        }
    }
    out
}

const PROCESS_FAMILY_PATTERNS: &[(&str, &[&str])] = &[
    (
        "discord",
        &[
            "discord.exe",
            "discordcanary",
            "discordptb",
            "discord\\",
            "discord/",
            "discord updater",
            "squirrel",
        ],
    ),
    (
        "chrome",
        &[
            "chrome.exe",
            "google chrome",
            "google\\chrome",
            "google/chrome",
            "googleupdate",
            "google\\update",
            "google/update",
            "chrome_proxy",
        ],
    ),
    (
        "msedge",
        &[
            "msedge.exe",
            "microsoft edge",
            "edgeupdate",
            "microsoft\\edge",
            "microsoft/edge",
            "msedgewebview2",
        ],
    ),
    (
        "firefox",
        &[
            "firefox.exe",
            "mozilla firefox",
            "mozilla\\firefox",
            "mozilla/firefox",
            "firefox installer",
        ],
    ),
    (
        "brave",
        &[
            "brave.exe",
            "bravesoftware\\brave-browser",
            "bravesoftware/brave-browser",
            "braveupdate",
        ],
    ),
    (
        "opera",
        &[
            "opera.exe",
            "opera gx",
            "operagx",
            "opera\\",
            "opera/",
            "launcher.exe opera",
        ],
    ),
    (
        "telegram",
        &[
            "telegram.exe",
            "telegram desktop",
            "telegram\\",
            "telegram/",
            "updater.exe telegram",
        ],
    ),
    (
        "steam",
        &[
            "steam.exe",
            "steamservice",
            "steamwebhelper",
            "steam\\",
            "steam/",
        ],
    ),
    (
        "spotify",
        &[
            "spotify.exe",
            "spotify\\",
            "spotify/",
            "spotifylauncher",
        ],
    ),
    (
        "code",
        &[
            "code.exe",
            "code helper",
            "visual studio code",
            "microsoft vs code",
            "vscode",
        ],
    ),
    (
        "teams",
        &[
            "teams.exe",
            "msteams",
            "microsoft teams",
            "teams\\",
            "teams/",
        ],
    ),
    (
        "riot",
        &[
            "riotclient",
            "riot client",
            "riot\\",
            "riot/",
            "valorant",
        ],
    ),
    (
        "epic",
        &[
            "epicgameslauncher",
            "epic games",
            "epic\\",
            "epic/",
        ],
    ),
];

fn normalized_process_haystack(name: &str, exe: &str) -> String {
    format!(
        "{} {} {}",
        name.to_lowercase(),
        exe.to_lowercase(),
        exe.to_lowercase().replace('\\', "/")
    )
}

fn normalize_hud_shortcut(shortcut: &str) -> String {
    let parts = shortcut
        .split('+')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let lower = part.to_lowercase();
            match lower.as_str() {
                "ctrl" | "control" => "CTRL".to_string(),
                "shift" => "SHIFT".to_string(),
                "alt" | "option" => "ALT".to_string(),
                "cmd" | "command" | "meta" | "win" | "super" => "SUPER".to_string(),
                "esc" => "ESCAPE".to_string(),
                _ if part.len() == 1 => part.to_uppercase(),
                _ => lower.to_uppercase(),
            }
        })
        .collect::<Vec<_>>();

    if parts.is_empty() {
        "CTRL+SHIFT+H".into()
    } else {
        parts.join("+")
    }
}

fn hud_bounds(app: &AppHandle, settings: &SettingsState) -> (f64, f64, f64, f64) {
    let (origin_x, origin_y, area_width, area_height) = app
        .get_webview_window("main")
        .and_then(|window| window.current_monitor().ok().flatten().or_else(|| window.primary_monitor().ok().flatten()))
        .map(|monitor| {
            let work = monitor.work_area().clone();
            (
                work.position.x as f64,
                work.position.y as f64,
                work.size.width as f64,
                work.size.height as f64,
            )
        })
        .unwrap_or((0.0, 0.0, 1920.0, 1080.0));

    let scale = (settings.hud_scale as f64 / 100.0).clamp(0.7, 1.5);
    let width = ((settings.hud_width as f64 * scale) + 24.0).round().max(280.0);
    let height = ((settings.hud_height as f64 * scale) + 24.0).round().max(160.0);
    let margin = 22.0;
    let max_x = (origin_x + area_width - width).max(origin_x);
    let max_y = (origin_y + area_height - height).max(origin_y);

    if settings.hud_position_mode.eq_ignore_ascii_case("custom") {
        let x = (origin_x + settings.hud_x as f64).clamp(origin_x, max_x);
        let y = (origin_y + settings.hud_y as f64).clamp(origin_y, max_y);
        return (x, y, width, height);
    }

    let x = if settings.hud_corner.contains("right") {
        origin_x + area_width - width - margin
    } else {
        origin_x + margin
    };
    let y = if settings.hud_corner.contains("bottom") {
        origin_y + area_height - height - margin
    } else {
        origin_y + margin
    };
    (x.clamp(origin_x, max_x), y.clamp(origin_y, max_y), width, height)
}

fn sync_hud_window(app: &AppHandle, settings: &SettingsState, show: bool) -> Result<(), String> {
    if !settings.hud_enabled {
        if let Some(window) = app.get_webview_window("hud") {
            let _ = window.close();
        }
        return Ok(());
    }

    let (x, y, width, height) = hud_bounds(app, settings);

    let window = if let Some(window) = app.get_webview_window("hud") {
        window
    } else {
        WebviewWindowBuilder::new(app, "hud", WebviewUrl::App("index.html".into()))
            .title("Nocturne HUD")
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focusable(false)
            .focused(false)
            .visible(show)
            .position(x, y)
            .inner_size(width, height)
            .build()
            .map_err(|e| e.to_string())?
    };

    window
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    let _ = window.set_always_on_top(true);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_focusable(false);
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.set_resizable(false);
    let _ = window.set_decorations(false);

    if show {
        window.show().map_err(|e| e.to_string())?;
        let _ = app.emit_to("hud", "nocturne://hud-sync", ());
    } else {
        let _ = app.emit_to("hud", "nocturne://hud-sync", ());
    }

    Ok(())
}

fn toggle_hud_window_impl(app: &AppHandle, state: &Arc<SharedState>) -> Result<(), String> {
    let settings = state.settings.lock().unwrap().clone();
    if !settings.hud_enabled {
        if let Some(window) = app.get_webview_window("hud") {
            let _ = window.close();
        }
        return Ok(());
    }

    if let Some(window) = app.get_webview_window("hud") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    sync_hud_window(app, &settings, true)
}

fn sync_hud_shortcut(app: &AppHandle, state: &Arc<SharedState>, settings: &SettingsState) -> Result<(), String> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    let mut active = state.hud_shortcut.lock().unwrap();
    *active = None;

    if !settings.hud_enabled {
        return Ok(());
    }

    let shortcut = normalize_hud_shortcut(&settings.hud_hotkey);
    let shared = state.clone();
    manager
        .on_shortcut(shortcut.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = toggle_hud_window_impl(app, &shared);
            }
        })
        .map_err(|e| e.to_string())?;

    *active = Some(shortcut);
    Ok(())
}

fn process_family(name: &str, exe: &str) -> String {
    let n = name.to_lowercase();
    let e = exe.to_lowercase();
    let haystack = normalized_process_haystack(&n, &e);

    for (family, patterns) in PROCESS_FAMILY_PATTERNS {
        if patterns.iter().any(|pattern| haystack.contains(pattern)) {
            return (*family).to_string();
        }
    }

    if haystack.contains("explorer.exe") || haystack.contains("\\explorer") || haystack.contains("/explorer") {
        return "explorer".into();
    }
    if haystack.contains("obs64") || haystack.contains("obs.exe") || haystack.contains("obs-studio") {
        return "obs".into();
    }

    if let Some(stem) = Path::new(exe).file_stem().and_then(|s| s.to_str()) {
        if !stem.is_empty() {
            return stem.to_lowercase();
        }
    }
    n.trim_end_matches(".exe").to_string()
}

fn normalized_app_tokens(name: &str, path: &str) -> Vec<String> {
    let mut tokens = name
        .to_lowercase()
        .replace(['(', ')', '[', ']', ',', '.', '_'], " ")
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .map(|token| token.to_string())
        .collect::<Vec<_>>();

    let family = process_family(name, path);
    if !family.is_empty() {
        tokens.push(family);
    }

    if let Some(stem) = Path::new(path).file_stem().and_then(|stem| stem.to_str()) {
        if stem.len() > 2 {
            tokens.push(stem.to_lowercase());
        }
    }

    tokens.sort();
    tokens.dedup();
    tokens
}

fn matches_autostart_entry(name: &str, path: &str, item: &AutostartItem) -> bool {
    let haystack = format!("{} {}", item.name.to_lowercase(), item.path.to_lowercase());
    normalized_app_tokens(name, path)
        .into_iter()
        .any(|token| haystack.contains(&token))
}

#[cfg(windows)]
fn current_foreground_pid() -> Option<i64> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
        if pid == 0 { None } else { Some(pid as i64) }
    }
}

#[cfg(windows)]
fn current_foreground_bounds() -> Option<WindowBounds> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut rect as *mut RECT) == 0 {
            return None;
        }
        Some(WindowBounds {
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left).max(320),
            height: (rect.bottom - rect.top).max(240),
        })
    }
}

#[cfg(not(windows))]
fn current_foreground_pid() -> Option<i64> { None }
#[cfg(not(windows))]
fn current_foreground_bounds() -> Option<WindowBounds> { None }

#[cfg(windows)]
fn trim_and_set_priority(pid: i64, priority: u32) {
    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION | PROCESS_SET_QUOTA,
            0,
            pid as u32,
        );
        if !handle.is_null() {
            let _ = SetPriorityClass(handle, priority);
            let _ = K32EmptyWorkingSet(handle);
            let _ = CloseHandle(handle);
        }
    }
}

#[cfg(not(windows))]
fn trim_and_set_priority(_pid: i64, _priority: u32) {}

fn suspend_process(pid: i64) {
    let _ = powershell(&format!("Suspend-Process -Id {} -ErrorAction SilentlyContinue", pid));
}

fn resume_process(pid: i64) {
    let _ = powershell(&format!("Resume-Process -Id {} -ErrorAction SilentlyContinue", pid));
    #[cfg(windows)]
    trim_and_set_priority(pid, NORMAL_PRIORITY_CLASS);
}

fn apply_mode(pid: i64, mode: &str) {
    match mode {
        "Eco" => {
            #[cfg(windows)]
            trim_and_set_priority(pid, IDLE_PRIORITY_CLASS);
        }
        "Balanced" => {
            #[cfg(windows)]
            trim_and_set_priority(pid, BELOW_NORMAL_PRIORITY_CLASS);
        }
        "Freeze" => suspend_process(pid),
        _ => {}
    }
}

fn match_rule<'a>(rules: &'a [OptimizationRule], process_name: &str, exe: &str) -> Option<&'a OptimizationRule> {
    let name = process_name.to_lowercase();
    let exe_lower = exe.to_lowercase();
    let family = process_family(&name, &exe_lower);
    rules.iter().find(|r| {
        if !r.enabled {
            return false;
        }
        let key = r.process_name.to_lowercase().trim_end_matches(".exe").to_string();
        let family_key = if r.family_key.trim().is_empty() {
            key.clone()
        } else {
            r.family_key.to_lowercase()
        };
        name.contains(&key)
            || exe_lower.contains(&key)
            || family == family_key
            || family.contains(&family_key)
            || key == family
    })
}

fn collect_snapshot(
    system: &mut System,
    rules: &[OptimizationRule],
    optimized: &HashMap<i64, String>,
) -> SystemSnapshot {
    system.refresh_memory();
    system.refresh_cpu_usage();
    system.refresh_processes_specifics(
        ProcessRefreshKind::new()
            .with_cpu()
            .with_memory()
            .with_exe(UpdateKind::OnlyIfNotSet),
    );

    let fg = current_foreground_pid();

    let mut processes = system
        .processes()
        .iter()
        .map(|(pid, proc_)| {
            let exe_string = proc_.exe().map(|p| p.display().to_string()).unwrap_or_else(String::new);
            let rule = match_rule(rules, proc_.name(), &exe_string);
            ProcessInfo {
                pid: pid.as_u32() as i64,
                name: proc_.name().to_string().to_lowercase(),
                display_name: pretty_name(proc_.name()),
                exe: exe_string.clone(),
                cpu: proc_.cpu_usage(),
                memory_mb: (proc_.memory() as f64) / 1024.0 / 1024.0,
                status: format!("{:?}", proc_.status()),
                foreground: fg == Some(pid.as_u32() as i64),
                optimizable: true,
                optimized_state: optimized
                    .get(&(pid.as_u32() as i64))
                    .cloned()
                    .unwrap_or_else(|| "Normal".to_string()),
                icon_hint: icon_hint_for_name(proc_.name()),
                rule_matched: rule.map(|r| r.id.clone()),
            }
        })
        .collect::<Vec<_>>();

    processes.sort_by(|a, b| {
        b.foreground
            .cmp(&a.foreground)
            .then_with(|| b.rule_matched.is_some().cmp(&a.rule_matched.is_some()))
            .then_with(|| {
                b.cpu
                    .partial_cmp(&a.cpu)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                b.memory_mb
                    .partial_cmp(&a.memory_mb)
                    .unwrap_or(Ordering::Equal)
            })
    });

    if processes.len() > 220 {
        processes.truncate(220);
    }

    processes.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(Ordering::Equal)
    });

    let total_mem = system.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let used_mem = system.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let total_swap = system.total_swap() as f64 / 1024.0 / 1024.0 / 1024.0;
    let used_swap = system.used_swap() as f64 / 1024.0 / 1024.0 / 1024.0;

    SystemSnapshot {
        cpu_usage: system.global_cpu_info().cpu_usage(),
        ram_used_gb: used_mem,
        ram_total_gb: total_mem,
        swap_used_gb: used_swap,
        swap_total_gb: total_swap,
        uptime_seconds: System::uptime(),
        foreground_pid: fg,
        processes,
    }
}

fn monitor_loop(app: AppHandle, shared: Arc<SharedState>) {
    std::thread::spawn(move || loop {
        let rules = shared.rules.lock().unwrap().clone();
        let settings = shared.settings.lock().unwrap().clone();
        let security = shared.security_config.lock().unwrap().clone();
        let mut optimized = shared.optimized.lock().unwrap();
        let mut system = shared.system.lock().unwrap();
        let snapshot = collect_snapshot(&mut system, &rules, &optimized);
        drop(system);
        *shared.snapshot_cache.lock().unwrap() = snapshot.clone();

        if settings.auto_apply_rules {
            for process in &snapshot.processes {
                if let Some(rule) = match_rule(&rules, &process.name, &process.exe) {
                    if process.foreground && rule.auto_resume {
                        if optimized.remove(&process.pid).is_some() {
                            resume_process(process.pid);
                        }
                    } else if !process.foreground && (!rule.require_background || !process.foreground) {
                        let current = optimized.get(&process.pid).cloned();
                        if current.as_deref() != Some(rule.mode.as_str()) {
                            apply_mode(process.pid, &rule.mode);
                            optimized.insert(process.pid, rule.mode.clone());
                        }
                    }
                } else if optimized.remove(&process.pid).is_some() {
                    resume_process(process.pid);
                }
            }
        }
        drop(optimized);

        let popular_families = [
            "discord", "chrome", "msedge", "firefox", "brave", "opera", "telegram", "steam",
            "spotify", "code",
        ];
        let mut detected = snapshot
            .processes
            .iter()
            .map(|p| process_family(&p.name, &p.exe))
            .filter(|family| popular_families.contains(&family.as_str()))
            .map(|family| format!("{family}.exe"))
            .collect::<Vec<_>>();
        detected.sort();
        detected.dedup();

        let fg_name = snapshot
            .processes
            .iter()
            .find(|p| p.foreground)
            .map(|p| p.name.clone());
        let fg_bounds = current_foreground_bounds();

        {
            let mut last = shared.last_foreground.lock().unwrap();
            let mut armed = shared.armed_apps.lock().unwrap();
            let mut runtime = shared.security_runtime.lock().unwrap();
            runtime.present_popular_apps = detected;
            if runtime.locked && runtime.locked_app.is_some() {
                runtime.overlay_bounds = current_foreground_bounds();
            }

            if security.lock_enabled && security.password_hash.is_some() {
                if let Some(previous) = last.clone() {
                    if Some(previous.clone()) != fg_name && security.protected_apps.contains(&previous) {
                        armed.insert(previous, Instant::now());
                    }
                }

                if let Some(current) = fg_name.clone() {
                    if security.protected_apps.contains(&current) {
                        if let Some(armed_since) = armed.get(&current).cloned() {
                            let elapsed_minutes = armed_since.elapsed().as_secs() / 60;
                            let grace_ok = elapsed_minutes >= security.grace_minutes as u64;
                            let should_lock = (security.lock_on_activate || security.lock_on_restore) && grace_ok;
                            if should_lock {
                                runtime.locked = true;
                                runtime.locked_app = Some(current.clone());
                                runtime.overlay_bounds = fg_bounds.clone();
                                armed.remove(&current);
                                let _ = app.emit("guard-lock", &current);
                            }
                        }
                    }
                }
            }
            *last = fg_name;
        }

        let mut sleep_ms = settings.refresh_ms.max(2200).min(6000);
        if settings.aggressive_mode {
            sleep_ms = sleep_ms.saturating_sub(600).max(1600);
        }
        std::thread::sleep(Duration::from_millis(sleep_ms));
    });
}

#[tauri::command]
fn get_system_snapshot(state: State<'_, Arc<SharedState>>) -> SystemSnapshot {
    let cached = state.snapshot_cache.lock().unwrap().clone();
    if !cached.processes.is_empty() {
        return cached;
    }
    let rules = state.rules.lock().unwrap().clone();
    let optimized = state.optimized.lock().unwrap();
    let mut system = state.system.lock().unwrap();
    collect_snapshot(&mut system, &rules, &optimized)
}

#[tauri::command]
fn get_rules(state: State<'_, Arc<SharedState>>) -> Vec<OptimizationRule> {
    state.rules.lock().unwrap().clone()
}

#[tauri::command]
fn save_rules(rules: Vec<OptimizationRule>, state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    *state.rules.lock().unwrap() = rules.clone();
    save_json(&rules_path(), &rules)
}

fn list_autostart_items_impl() -> Result<Vec<AutostartItem>, String> {
    #[cfg(not(windows))]
    {
        Ok(vec![])
    }
    #[cfg(windows)]
    {
        use winreg::{enums::*, RegKey};
        let mut items: Vec<AutostartItem> = vec![];

        let reg_targets = vec![
            (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run", "HKCU Run"),
            (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce", "HKCU RunOnce"),
            (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer\\Run", "HKCU Policy Run"),
            (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\RunServices", "HKCU RunServices"),
            (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\RunServicesOnce", "HKCU RunServicesOnce"),
            (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Run", "HKLM Run"),
            (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce", "HKLM RunOnce"),
            (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer\\Run", "HKLM Policy Run"),
            (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\RunServices", "HKLM RunServices"),
            (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\RunServicesOnce", "HKLM RunServicesOnce"),
            (HKEY_LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run", "HKLM WOW64 Run"),
            (HKEY_LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\RunOnce", "HKLM WOW64 RunOnce"),
            (HKEY_LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer\\Run", "HKLM WOW64 Policy Run"),
        ];

        for (hive, path, label) in reg_targets {
            let root = RegKey::predef(hive);
            if let Ok(key) = root.open_subkey(path) {
                for entry in key.enum_values().flatten() {
                    let name = entry.0;
                    let value = key.get_value::<String, _>(&name)
                        .ok()
                        .or_else(|| key.get_value::<u32, _>(&name).ok().map(|v| v.to_string()))
                        .unwrap_or_default();
                    items.push(AutostartItem {
                        id: format!("{}::{}::Registry", label, name),
                        source: label.into(),
                        name: name.clone(),
                        path: value,
                        item_type: "Registry".into(),
                        enabled: true,
                        details: format!(r"{}\{}", if hive == HKEY_CURRENT_USER { "HKCU" } else { "HKLM" }, path),
                        icon_hint: icon_hint_for_name(&name),
                    });
                }
            }
        }

        let startup_user = dirs::data_dir().unwrap_or_default().join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        let startup_common = PathBuf::from(std::env::var("ProgramData").unwrap_or_default()).join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        for (label, dir) in [("Startup User", startup_user), ("Startup Common", startup_common)] {
            if dir.exists() {
                if let Ok(read_dir) = fs::read_dir(&dir) {
                    for entry in read_dir.flatten() {
                        let path = entry.path();
                        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("startup item").to_string();
                        let enabled = !path.to_string_lossy().to_lowercase().ends_with(".disabled");
                        items.push(AutostartItem {
                            id: format!("{}::{}::StartupFolder", label, name),
                            source: label.into(),
                            name: name.clone(),
                            path: path.display().to_string(),
                            item_type: "StartupFolder".into(),
                            enabled,
                            details: dir.display().to_string(),
                            icon_hint: icon_hint_for_name(&name),
                        });
                    }
                }
            }
        }

        let task_script = r#"
$items = Get-ScheduledTask -ErrorAction SilentlyContinue |
  Where-Object { $_.TaskPath -notlike '\Microsoft*' } |
  ForEach-Object {
    $action = $_.Actions | Select-Object -First 1
    [PSCustomObject]@{
      id = "Scheduled Task::$($_.TaskName)::Task"
      source = 'Scheduled Task'
      name = $_.TaskName
      path = ([string]$action.Execute + ' ' + [string]$action.Arguments).Trim()
      itemType = 'Task'
      enabled = ($_.State -ne 'Disabled')
      details = $_.TaskPath
      iconHint = $_.TaskName
    }
  }
@($items) | ConvertTo-Json -Depth 5 -Compress
"#;
        if let Ok(out) = powershell(task_script) {
            let mut more: Vec<AutostartItem> = serde_json::from_str(&out).unwrap_or_default();
            items.append(&mut more);
        }

        let service_script = r#"
$items = Get-CimInstance Win32_Service -ErrorAction SilentlyContinue |
  Where-Object { $_.StartMode -in @('Auto','Automatic','Disabled') } |
  Select-Object -First 120 |
  ForEach-Object {
    [PSCustomObject]@{
      id = "Service::$($_.Name)::Service"
      source = 'Service'
      name = $_.Name
      path = [string]$_.PathName
      itemType = 'Service'
      enabled = ($_.StartMode -ne 'Disabled')
      details = [string]$_.DisplayName
      iconHint = $_.Name
    }
  }
@($items) | ConvertTo-Json -Depth 5 -Compress
"#;
        if let Ok(out) = powershell(service_script) {
            let mut more: Vec<AutostartItem> = serde_json::from_str(&out).unwrap_or_default();
            items.append(&mut more);
        }

        items.sort_by(|a, b| a.source.to_lowercase().cmp(&b.source.to_lowercase()).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        items.dedup_by(|a, b| a.item_type == b.item_type && a.name.eq_ignore_ascii_case(&b.name) && a.path.eq_ignore_ascii_case(&b.path));
        Ok(items)
    }
}

#[tauri::command]
fn list_autostart_items() -> Result<Vec<AutostartItem>, String> {
    list_autostart_items_impl()
}

#[tauri::command]
fn toggle_autostart_item(item: AutostartItem, enable: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = item;
        let _ = enable;
        return Ok(());
    }
    #[cfg(windows)]
    {
        match item.item_type.as_str() {
            "Registry" => {
                let path = psq(&item.details);
                let name = psq(&item.name);
                if enable {
                    let value = psq(&item.path);
                    powershell(&format!("Set-ItemProperty -Path \"{}\" -Name \"{}\" -Value \"{}\"", path, name, value))?;
                } else {
                    powershell(&format!("Remove-ItemProperty -Path \"{}\" -Name \"{}\" -ErrorAction SilentlyContinue", path, name))?;
                }
            }
            "StartupFolder" => {
                let file = PathBuf::from(&item.path);
                if enable {
                    let from = if file.exists() { file.clone() } else { PathBuf::from(format!("{}.disabled", item.path)) };
                    let to = if from.extension().and_then(|e| e.to_str()) == Some("disabled") {
                        PathBuf::from(item.path.trim_end_matches(".disabled"))
                    } else { file.clone() };
                    if from != to { fs::rename(from, to).map_err(|e| e.to_string())?; }
                } else {
                    let disabled = PathBuf::from(format!("{}.disabled", item.path));
                    if file.exists() { fs::rename(file, disabled).map_err(|e| e.to_string())?; }
                }
            }
            "Task" => {
                let name = psq(&item.name);
                let task_path = psq(&item.details);
                let cmd = if enable {
                    format!("Enable-ScheduledTask -TaskName \"{}\" -TaskPath \"{}\"", name, task_path)
                } else {
                    format!("Disable-ScheduledTask -TaskName \"{}\" -TaskPath \"{}\"", name, task_path)
                };
                powershell(&cmd)?;
            }
            "Service" => {
                let name = psq(&item.name);
                let cmd = if enable {
                    format!("Set-Service -Name \"{}\" -StartupType Automatic", name)
                } else {
                    format!("Stop-Service -Name \"{}\" -Force -ErrorAction SilentlyContinue; Set-Service -Name \"{}\" -StartupType Disabled", name, name)
                };
                powershell(&cmd)?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(windows)]
fn read_reg_string(path: &str, name: &str) -> Option<String> {
    use winreg::{enums::*, RegKey};
    let (hive, subpath) = if let Some(rest) = path.strip_prefix("HKLM\\") {
        (RegKey::predef(HKEY_LOCAL_MACHINE), rest)
    } else if let Some(rest) = path.strip_prefix("HKCU\\") {
        (RegKey::predef(HKEY_CURRENT_USER), rest)
    } else {
        return None;
    };
    let key = hive.open_subkey(subpath).ok()?;
    key.get_value::<String, _>(name)
        .ok()
        .or_else(|| key.get_value::<u32, _>(name).ok().map(|v| v.to_string()))
}

#[cfg(not(windows))]
fn read_reg_string(_path: &str, _name: &str) -> Option<String> { None }

#[tauri::command]
fn get_registry_health() -> Vec<RegistryHealthItem> {
    let checks = vec![
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System", "EnableLUA", "1", "critical", "UAC bazowo włączony"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System", "PromptOnSecureDesktop", "1", "critical", "Monit UAC na bezpiecznym pulpicie"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System", "ConsentPromptBehaviorAdmin", "5", "high", "Rozsądny prompt dla admina"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System", "EnableVirtualization", "1", "medium", "Wirtualizacja UAC dla starszych aplikacji"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System", "FilterAdministratorToken", "1", "medium", "Admin Approval Mode dla wbudowanego admina"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System", "ValidateAdminCodeSignatures", "0", "medium", "Kod admina bez wymuszonego podpisu"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Lsa", "RunAsPPL", "1", "critical", "LSA Protection"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Lsa", "LimitBlankPasswordUse", "1", "high", "Brak pustych haseł"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Lsa", "NoLMHash", "1", "high", "Brak LM Hash"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Lsa", "LmCompatibilityLevel", "5", "high", "Silniejszy NTLM"),
        (r"HKLM\SOFTWARE\Microsoft\Windows Defender\Features", "TamperProtection", "5", "critical", "Defender Tamper Protection"),
        (r"HKLM\SOFTWARE\Microsoft\Windows Defender\Spynet", "SpyNetReporting", "2", "medium", "Cloud-delivered protection"),
        (r"HKLM\SOFTWARE\Microsoft\Windows Defender\Real-Time Protection", "DisableRealtimeMonitoring", "0", "critical", "Realtime Monitoring"),
        (r"HKLM\SOFTWARE\Microsoft\Windows Defender\Windows Defender Exploit Guard\Controlled Folder Access", "EnableControlledFolderAccess", "1", "medium", "Controlled Folder Access"),
        (r"HKLM\SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\StandardProfile", "EnableFirewall", "1", "critical", "Firewall profil standard"),
        (r"HKLM\SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\PublicProfile", "EnableFirewall", "1", "critical", "Firewall profil publiczny"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer", "SmartScreenEnabled", "RequireAdmin", "medium", "SmartScreen dla pobranych plików"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System", "EnableSmartScreen", "1", "medium", "SmartScreen polityki systemowej"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System", "ShellSmartScreenLevel", "Block", "medium", "Poziom SmartScreen"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Terminal Server", "fDenyTSConnections", "1", "high", "Zdalny pulpit wyłączony"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Windows NT\Terminal Services", "fDenyTSConnections", "1", "high", "Polityka RDP wyłączona"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Remote Assistance", "fAllowToGetHelp", "0", "medium", "Remote Assistance wyłączone"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Attachments", "SaveZoneInformation", "2", "medium", "Mark of the Web aktywny"),
        (r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings", "DisablePasswordCaching", "1", "low", "Brak cache haseł IE legacy"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management", "ClearPageFileAtShutdown", "1", "low", "Czyszczenie pagefile przy shutdown"),
        (r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\kernel", "DisableExceptionChainValidation", "0", "medium", "SEHOP validation"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer", "NoAutorun", "1", "medium", "Autorun nośników wyłączony"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer", "NoDriveTypeAutoRun", "255", "medium", "AutoRun all drives disabled"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Windows\Installer", "AlwaysInstallElevated", "0", "critical", "AlwaysInstallElevated off (machine)"),
        (r"HKCU\SOFTWARE\Policies\Microsoft\Windows\Installer", "AlwaysInstallElevated", "0", "critical", "AlwaysInstallElevated off (user)"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Windows\PowerShell", "EnableScripts", "0", "low", "PowerShell scripts nie wymuszane polityką"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Windows\PowerShell\ScriptBlockLogging", "EnableScriptBlockLogging", "1", "medium", "Script Block Logging"),
        (r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System", "LocalAccountTokenFilterPolicy", "0", "medium", "Remote UAC token filtering"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System", "EnableLUA", "1", "critical", "Polityka UAC"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\Biometrics", "Enabled", "1", "low", "Biometria włączona jeśli używana"),
        (r"HKLM\SOFTWARE\Policies\Microsoft\FVE", "UseAdvancedStartup", "1", "low", "BitLocker advanced startup"),
    ];

    checks.into_iter().map(|(path, value, recommended, severity, meaning)| {
        let current = read_reg_string(path, value).unwrap_or_else(|| "brak / n.d.".to_string());
        RegistryHealthItem {
            key_path: path.to_string(),
            value_name: value.to_string(),
            current: current.clone(),
            recommended: recommended.to_string(),
            healthy: current.eq_ignore_ascii_case(recommended),
            meaning: meaning.to_string(),
            severity: severity.to_string(),
        }
    }).collect()
}

#[tauri::command]
fn run_registry_audit_console(mode: String) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = mode;
        Ok(())
    }
    #[cfg(windows)]
    {
        let action = if mode.eq_ignore_ascii_case("repair") { "repair" } else { "scan" };
        let script = r#"
$mode = '__MODE__'
$checks = @(
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System','EnableLUA','1','UAC bazowo włączony'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System','PromptOnSecureDesktop','1','UAC na bezpiecznym pulpicie'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System','ConsentPromptBehaviorAdmin','5','Prompt admina'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System','EnableVirtualization','1','Wirtualizacja UAC'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System','FilterAdministratorToken','1','Admin Approval Mode'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Lsa','RunAsPPL','1','LSA Protection'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Lsa','LimitBlankPasswordUse','1','Brak pustych haseł'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Lsa','NoLMHash','1','Brak LM Hash'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Lsa','LmCompatibilityLevel','5','Silniejszy NTLM'),
  @('HKLM:\SOFTWARE\Microsoft\Windows Defender\Features','TamperProtection','5','Tamper Protection'),
  @('HKLM:\SOFTWARE\Microsoft\Windows Defender\Real-Time Protection','DisableRealtimeMonitoring','0','Realtime monitoring'),
  @('HKLM:\SOFTWARE\Microsoft\Windows Defender\Spynet','SpyNetReporting','2','Cloud protection'),
  @('HKLM:\SOFTWARE\Microsoft\Windows Defender\Windows Defender Exploit Guard\Controlled Folder Access','EnableControlledFolderAccess','1','Controlled Folder Access'),
  @('HKLM:\SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\StandardProfile','EnableFirewall','1','Firewall standard'),
  @('HKLM:\SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\PublicProfile','EnableFirewall','1','Firewall publiczny'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer','SmartScreenEnabled','RequireAdmin','SmartScreen Explorer'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\System','EnableSmartScreen','1','SmartScreen policy'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\System','ShellSmartScreenLevel','Block','SmartScreen level'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Terminal Server','fDenyTSConnections','1','RDP wyłączony'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Windows NT\Terminal Services','fDenyTSConnections','1','Polityka RDP'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Remote Assistance','fAllowToGetHelp','0','Remote Assistance'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Attachments','SaveZoneInformation','2','MOTW'),
  @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings','DisablePasswordCaching','1','Password caching legacy'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management','ClearPageFileAtShutdown','1','Clear page file'),
  @('HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\kernel','DisableExceptionChainValidation','0','SEHOP'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer','NoAutorun','1','Autorun disabled'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer','NoDriveTypeAutoRun','255','Autorun drives'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\Installer','AlwaysInstallElevated','0','AlwaysInstallElevated machine'),
  @('HKCU:\SOFTWARE\Policies\Microsoft\Windows\Installer','AlwaysInstallElevated','0','AlwaysInstallElevated user'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\PowerShell','EnableScripts','0','PowerShell scripts'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\PowerShell\ScriptBlockLogging','EnableScriptBlockLogging','1','ScriptBlockLogging'),
  @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System','LocalAccountTokenFilterPolicy','0','Remote UAC token filtering'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\System','EnableLUA','1','UAC policy'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\Biometrics','Enabled','1','Biometrics'),
  @('HKLM:\SOFTWARE\Policies\Microsoft\FVE','UseAdvancedStartup','1','BitLocker advanced startup')
)
Write-Host 'Nocturne Registry Audit Console'
Write-Host ('Tryb: ' + $mode)
Write-Host ('Liczba punktów: ' + $checks.Count)
foreach ($c in $checks) {
  $path = $c[0]; $name = $c[1]; $recommended = $c[2];
  Write-Host ('===== ' + $name + ' =====')
  $current = 'brak / n.d.'
  try { $value = (Get-ItemProperty -Path $path -Name $name -ErrorAction Stop).$name; if ($null -ne $value) { $current = [string]$value } } catch {}
  Write-Host ('ścieżka: ' + $path)
  Write-Host ('stan: ' + $current)
  if ($mode -eq 'repair' -and $current -ne $recommended) {
    try {
      New-Item -Path $path -Force | Out-Null
      $num = 0
      if ([int]::TryParse($recommended, [ref]$num)) {
        Set-ItemProperty -Path $path -Name $name -Type DWord -Value $num -ErrorAction Stop
      } else {
        Set-ItemProperty -Path $path -Name $name -Value $recommended -ErrorAction Stop
      }
      Write-Host ('naprawa: OK -> ' + $recommended)
    } catch {
      Write-Host ('naprawa: FAIL -> ' + $_.Exception.Message)
    }
  } else {
    Write-Host ('zalecane: ' + $recommended)
  }
  Write-Host ''
}
Write-Host '===== PODSUMOWANIE ====='
Write-Host 'Konsola zostaje otwarta. Zamknij ją ręcznie.'
"#.replace("__MODE__", action);
        Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-NoExit", "-Command", &script])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tauri::command]
fn run_offline_preset(preset_id: String) -> Result<OfflinePresetResult, String> {
    #[cfg(not(windows))]
    {
        return Ok(OfflinePresetResult { preset: preset_id, success: true, details: "Preset dostępny głównie na Windows.".into() });
    }
    #[cfg(windows)]
    {
        let details = match preset_id.as_str() {
            "clean_temp" => {
                powershell(r#"
$targets = @($env:TEMP, 'C:\Windows\Temp')
foreach ($p in $targets) { if (Test-Path $p) { Get-ChildItem $p -Force -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue } }
Write-Output 'Wyczyszczono katalogi tymczasowe.'
"#)?
            }
            "background_quiet" => {
                powershell(r#"
$services = @('SysMain','DiagTrack','WSearch')
foreach ($svc in $services) { try { Stop-Service -Name $svc -Force -ErrorAction SilentlyContinue } catch {} }
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Process explorer.exe
Write-Output 'Próbowano wyciszyć część usług tła i odświeżyć explorer.'
"#)?
            }
            "debloat_lite" => {
                powershell(r#"
New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager' -Force | Out-Null
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager' -Name 'SubscribedContent-338388Enabled' -Type DWord -Value 0 -ErrorAction SilentlyContinue
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager' -Name 'SystemPaneSuggestionsEnabled' -Type DWord -Value 0 -ErrorAction SilentlyContinue
Write-Output 'Wyłączono część sugestii i content delivery Windows.'
"#)?
            }
            _ => return Err("Nieznany preset".into()),
        };
        Ok(OfflinePresetResult { preset: preset_id, success: true, details: details.trim().to_string() })
    }
}

#[cfg(windows)]
fn fetch_windows_apps() -> Result<Vec<WindowsBundleApp>, String> {
    let script = r#"
$targets = @(
  'Microsoft.YourPhone',
  'Microsoft.Getstarted',
  'Microsoft.ZuneMusic',
  'Microsoft.ZuneVideo',
  'Microsoft.MicrosoftSolitaireCollection',
  'Microsoft.BingNews',
  'Microsoft.BingWeather',
  'Microsoft.People',
  'Microsoft.WindowsFeedbackHub',
  'Clipchamp.Clipchamp',
  'Microsoft.GamingApp',
  'Microsoft.XboxApp',
  'Microsoft.XboxGameCallableUI',
  'Microsoft.MicrosoftOfficeHub'
)
$out = foreach ($target in $targets) {
  $pkg = Get-AppxPackage -Name $target -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($pkg) {
    [PSCustomObject]@{
      id = $target
      name = $pkg.Name
      publisher = $pkg.Publisher
      path = $pkg.InstallLocation
      installed = $true
      removable = -not $pkg.NonRemovable
      status = 'Installed'
      startupEnabled = $false
      permissionsSummary = 'Store app / package sandbox'
      iconHint = $pkg.Name
    }
  } else {
    [PSCustomObject]@{
      id = $target
      name = $target
      publisher = 'Microsoft'
      path = ''
      installed = $false
      removable = $true
      status = 'Not detected'
      startupEnabled = $false
      permissionsSummary = 'Store app / package sandbox'
      iconHint = $target
    }
  }
}
@($out) | ConvertTo-Json -Depth 5 -Compress
"#;
    let out = powershell(script)?;
    if out.trim().is_empty() || out.trim() == "null" {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str(&out).unwrap_or_default())
}

#[cfg(not(windows))]
fn fetch_windows_apps() -> Result<Vec<WindowsBundleApp>, String> { Ok(vec![]) }

#[cfg(windows)]
fn fetch_installed_programs() -> Result<Vec<InstalledProgram>, String> {
    let script = r#"
$paths = @(
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$items = foreach ($path in $paths) {
  Get-ItemProperty $path -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName } | ForEach-Object {
    [PSCustomObject]@{
      id = ($_.PSChildName)
      name = $_.DisplayName
      publisher = ([string]$_.Publisher)
      version = ([string]$_.DisplayVersion)
      path = ([string]$_.DisplayIcon)
      startupEnabled = $false
      kind = if ($_.WindowsInstaller -eq 1) { 'MSI / desktop' } else { 'Desktop app' }
      permissionsSummary = 'Desktop app / user-context'
      iconHint = $_.DisplayName
    }
  }
}
@($items | Sort-Object name -Unique | Select-Object -First 180) | ConvertTo-Json -Depth 5 -Compress
"#;
    let out = powershell(script)?;
    if out.trim().is_empty() || out.trim() == "null" {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str(&out).unwrap_or_default())
}

#[cfg(not(windows))]
fn fetch_installed_programs() -> Result<Vec<InstalledProgram>, String> { Ok(vec![]) }

#[tauri::command]
fn get_app_inventory() -> Result<AppInventory, String> {
    let autostart = list_autostart_items_impl().unwrap_or_default();
    let mut windows_apps = fetch_windows_apps()?;
    let mut installed_programs = fetch_installed_programs()?;

    for app in &mut windows_apps {
        app.startup_enabled = autostart
            .iter()
            .any(|item| matches_autostart_entry(&app.name, &app.path, item));
        app.icon_hint = icon_hint_for_name(&app.name);
    }

    for app in &mut installed_programs {
        app.startup_enabled = autostart
            .iter()
            .any(|item| matches_autostart_entry(&app.name, &app.path, item));
        app.icon_hint = icon_hint_for_name(&app.name);
    }

    Ok(AppInventory {
        windows_apps,
        installed_programs,
    })
}

#[tauri::command]
fn get_security_config(state: State<'_, Arc<SharedState>>) -> SecurityConfigView {
    let config = state.security_config.lock().unwrap().clone();
    SecurityConfigView {
        password_set: config.password_hash.is_some(),
        file_protection: config.file_protection,
        protected_apps: config.protected_apps,
        lock_enabled: config.lock_enabled,
        lock_on_restore: config.lock_on_restore,
        lock_on_activate: config.lock_on_activate,
        grace_minutes: config.grace_minutes,
        app_password_on_start: config.app_password_on_start,
    }
}

#[tauri::command]
fn save_security_config(payload: SecurityConfigPayload, state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    let mut config = state.security_config.lock().unwrap();
    if let Some(password) = payload.password {
        if !password.trim().is_empty() {
            config.password_hash = Some(hash_password(&normalize_password(&password)));
        }
    }
    config.file_protection = payload.file_protection;
    config.protected_apps = payload
        .protected_apps
        .into_iter()
        .map(|x| x.to_lowercase())
        .collect();
    config.lock_enabled = payload.lock_enabled;
    config.lock_on_restore = payload.lock_on_restore;
    config.lock_on_activate = payload.lock_on_activate;
    config.grace_minutes = payload.grace_minutes;
    config.app_password_on_start = payload.app_password_on_start;

    if config.file_protection {
        let _ = fs::write(vault_path(), b"nocturne-vault-enabled");
    }

    save_json(&security_path(), &*config)
}

#[tauri::command]
fn get_security_runtime(state: State<'_, Arc<SharedState>>) -> SecurityRuntime {
    state.security_runtime.lock().unwrap().clone()
}

#[tauri::command]
fn unlock_guard(password: String, state: State<'_, Arc<SharedState>>) -> bool {
    let config = state.security_config.lock().unwrap().clone();
    if verify_password_value(&config.password_hash, &password) {
        let mut runtime = state.security_runtime.lock().unwrap();
        runtime.locked = false;
        runtime.locked_app = None;
        runtime.overlay_bounds = None;
        true
    } else {
        false
    }
}

#[tauri::command]
fn verify_password(password: String, state: State<'_, Arc<SharedState>>) -> bool {
    let config = state.security_config.lock().unwrap().clone();
    verify_password_value(&config.password_hash, &password)
}

#[tauri::command]
fn get_settings(state: State<'_, Arc<SharedState>>) -> SettingsState {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    settings: SettingsState,
    app: AppHandle,
    state: State<'_, Arc<SharedState>>,
) -> Result<(), String> {
    *state.settings.lock().unwrap() = settings.clone();
    save_json(&settings_path(), &settings)?;
    sync_hud_shortcut(&app, state.inner(), &settings)?;
    let should_show = app
        .get_webview_window("hud")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(settings.hud_enabled);
    sync_hud_window(&app, &settings, should_show)?;
    Ok(())
}

#[tauri::command]
fn toggle_hud_window(app: AppHandle, state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    toggle_hud_window_impl(&app, state.inner())
}

#[cfg(windows)]
fn fetch_network_adapters() -> Result<Vec<NetworkAdapter>, String> {
    let script = r#"
$items = Get-NetAdapterStatistics -ErrorAction SilentlyContinue | ForEach-Object {
  $adapter = Get-NetAdapter -Name $_.Name -ErrorAction SilentlyContinue
  $ip = Get-NetIPConfiguration -InterfaceAlias $_.Name -ErrorAction SilentlyContinue
  [PSCustomObject]@{
    name = $_.Name
    status = ([string]$adapter.Status)
    linkSpeed = ([string]$adapter.LinkSpeed)
    macAddress = ([string]$adapter.MacAddress)
    ipv4 = ([string]($ip.IPv4Address | Select-Object -First 1).IPAddress)
    sentMb = [math]::Round(($_.SentBytes / 1MB), 2)
    receivedMb = [math]::Round(($_.ReceivedBytes / 1MB), 2)
  }
}
@($items) | ConvertTo-Json -Depth 5 -Compress
"#;
    let out = powershell(script)?;
    if out.trim().is_empty() || out.trim() == "null" {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str(&out).unwrap_or_default())
}

#[cfg(not(windows))]
fn fetch_network_adapters() -> Result<Vec<NetworkAdapter>, String> { Ok(vec![]) }

#[tauri::command]
fn get_network_overview(state: State<'_, Arc<SharedState>>) -> Result<NetworkOverview, String> {
    Ok(NetworkOverview {
        adapters: fetch_network_adapters()?,
        rules: state.network_rules.lock().unwrap().clone(),
    })
}

#[tauri::command]
fn save_network_rules(rules: Vec<NetworkRule>, state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    *state.network_rules.lock().unwrap() = rules.clone();
    save_json(&network_rules_path(), &rules)
}

#[tauri::command]
fn run_network_tune() -> Result<NetworkTuneResult, String> {
    #[cfg(not(windows))]
    {
        return Ok(NetworkTuneResult { success: true, summary: "Dostępne głównie na Windows.".into() });
    }
    #[cfg(windows)]
    {
        let output = powershell(r#"
ipconfig /flushdns | Out-Host
netsh interface tcp set global autotuninglevel=normal | Out-Host
netsh interface tcp set heuristics disabled | Out-Host
Write-Output 'Zastosowano flush DNS, normal autotuning i wyłączono heurystyki TCP.'
"#)?;
        Ok(NetworkTuneResult { success: true, summary: output })
    }
}

#[cfg(windows)]
fn self_autostart_name() -> &'static str { "NocturneOptimizer" }

#[cfg(windows)]
fn get_self_autostart_impl() -> bool {
    use winreg::{enums::*, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .ok()
        .and_then(|key| key.get_value::<String, _>(self_autostart_name()).ok())
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn get_self_autostart_impl() -> bool { false }

#[tauri::command]
fn get_self_autostart() -> bool {
    get_self_autostart_impl()
}

#[tauri::command]
fn set_self_autostart(enable: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = enable;
        Ok(())
    }
    #[cfg(windows)]
    {
        use winreg::{enums::*, RegKey};
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .map_err(|e| e.to_string())?;
        if enable {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let cmd = format!("\"{}\"", exe.display());
            key.set_value(self_autostart_name(), &cmd).map_err(|e| e.to_string())?;
        } else {
            let _ = key.delete_value(self_autostart_name());
        }
        Ok(())
    }
}

fn main() {
    let shared = Arc::new(SharedState::default());
    *shared.rules.lock().unwrap() = load_json::<Vec<OptimizationRule>>(&rules_path());

    *shared.security_config.lock().unwrap() = load_json::<SecurityConfig>(&security_path());
    *shared.settings.lock().unwrap() = load_json::<SettingsState>(&settings_path());
    *shared.network_rules.lock().unwrap() = load_json::<Vec<NetworkRule>>(&network_rules_path());
    *shared.snapshot_cache.lock().unwrap() = SystemSnapshot {
        cpu_usage: 0.0,
        ram_used_gb: 0.0,
        ram_total_gb: 0.0,
        swap_used_gb: 0.0,
        swap_total_gb: 0.0,
        uptime_seconds: 0,
        foreground_pid: None,
        processes: vec![],
    };

    tauri::Builder::default()
        .manage(shared.clone())
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
            let settings = shared.settings.lock().unwrap().clone();
            sync_hud_shortcut(&app.handle(), &shared, &settings)?;
            sync_hud_window(&app.handle(), &settings, settings.hud_enabled)?;
            monitor_loop(app.handle().clone(), shared.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_snapshot,
            get_rules,
            save_rules,
            list_autostart_items,
            toggle_autostart_item,
            run_offline_preset,
            get_registry_health,
            get_app_inventory,
            get_security_config,
            save_security_config,
            get_security_runtime,
            unlock_guard,
            verify_password,
            get_settings,
            save_settings,
            toggle_hud_window,
            get_network_overview,
            save_network_rules,
            run_network_tune,
            run_registry_audit_console,
            get_self_autostart,
            set_self_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
