#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System};
use tauri::{AppHandle, Emitter, State};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND},
    System::{
        ProcessStatus::K32EmptyWorkingSet,
        Threading::{
            OpenProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS,
            NORMAL_PRIORITY_CLASS, PROCESS_QUERY_INFORMATION, PROCESS_SET_INFORMATION,
            PROCESS_SET_QUOTA,
        },
    },
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProcessInfo {
    pid: i64,
    name: String,
    exe: String,
    cpu: f32,
    memory_mb: f64,
    status: String,
    foreground: bool,
    optimizable: bool,
    optimized_state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[serde(rename_all = "camelCase")]
struct OptimizationRule {
    id: String,
    process_name: String,
    mode: String,
    require_background: bool,
    auto_resume: bool,
    enabled: bool,
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
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct SecurityConfig {
    password_hash: Option<String>,
    file_protection: bool,
    protected_apps: Vec<String>,
    lock_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecurityConfigView {
    password_set: bool,
    file_protection: bool,
    protected_apps: Vec<String>,
    lock_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SecurityConfigPayload {
    password: Option<String>,
    file_protection: bool,
    protected_apps: Vec<String>,
    lock_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct SecurityRuntime {
    locked: bool,
    locked_app: Option<String>,
    present_popular_apps: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SettingsState {
    refresh_ms: u64,
    auto_apply_rules: bool,
    aggressive_mode: bool,
    minimize_to_tray: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            refresh_ms: 1500,
            auto_apply_rules: true,
            aggressive_mode: false,
            minimize_to_tray: true,
        }
    }
}

#[derive(Default)]
struct SharedState {
    rules: Mutex<Vec<OptimizationRule>>,
    optimized: Mutex<HashMap<i64, String>>,
    security_config: Mutex<SecurityConfig>,
    security_runtime: Mutex<SecurityRuntime>,
    settings: Mutex<SettingsState>,
    armed_apps: Mutex<HashSet<String>>,
    last_foreground: Mutex<Option<String>>,
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

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    B64.encode(hasher.finalize())
}

fn powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn psq(input: &str) -> String {
    input.replace('"', "``\"")
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

        if pid == 0 {
            None
        } else {
            Some(pid as i64)
        }
    }
}

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
fn current_foreground_pid() -> Option<i64> {
    None
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

fn collect_snapshot(optimized: &HashMap<i64, String>) -> SystemSnapshot {
    let mut system = System::new_with_specifics(
        RefreshKind::new()
            .with_memory(MemoryRefreshKind::everything())
            .with_cpu(CpuRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_all();

    let fg = current_foreground_pid();

    let mut processes = system
        .processes()
        .iter()
        .map(|(pid, proc_)| ProcessInfo {
            pid: pid.as_u32() as i64,
            name: proc_.name().to_string().to_lowercase(),
            exe: proc_
                .exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(String::new),
            cpu: proc_.cpu_usage(),
            memory_mb: (proc_.memory() as f64) / 1024.0 / 1024.0,
            status: format!("{:?}", proc_.status()),
            foreground: fg == Some(pid.as_u32() as i64),
            optimizable: true,
            optimized_state: optimized
                .get(&(pid.as_u32() as i64))
                .cloned()
                .unwrap_or_else(|| "Normal".to_string()),
        })
        .collect::<Vec<_>>();

    processes.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
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
fn match_rule<'a>(rules: &'a [OptimizationRule], process_name: &str) -> Option<&'a OptimizationRule> {
    let name = process_name.to_lowercase();
    rules.iter()
        .find(|r| r.enabled && name.contains(&r.process_name.to_lowercase()))
}

fn monitor_loop(app: AppHandle, shared: Arc<SharedState>) {
    std::thread::spawn(move || loop {
        let rules = shared.rules.lock().unwrap().clone();
        let settings = shared.settings.lock().unwrap().clone();
        let mut optimized = shared.optimized.lock().unwrap();
        let snapshot = collect_snapshot(&optimized);

        if settings.auto_apply_rules {
            for process in &snapshot.processes {
                if let Some(rule) = match_rule(&rules, &process.name) {
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

        let popular = ["discord.exe", "chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "telegram.exe", "steam.exe"];
        let names: HashSet<String> = snapshot.processes.iter().map(|p| p.name.clone()).collect();
        let detected = popular
            .iter()
            .filter(|name| names.contains(**name))
            .map(|name| name.to_string())
            .collect::<Vec<_>>();

        let fg_name = snapshot
            .processes
            .iter()
            .find(|p| p.foreground)
            .map(|p| p.name.clone());

        let security = shared.security_config.lock().unwrap().clone();
        {
            let mut last = shared.last_foreground.lock().unwrap();
            let mut armed = shared.armed_apps.lock().unwrap();
            let mut runtime = shared.security_runtime.lock().unwrap();
            runtime.present_popular_apps = detected;

            if security.lock_enabled {
                if let Some(previous) = last.clone() {
                    if Some(previous.clone()) != fg_name && security.protected_apps.contains(&previous) {
                        armed.insert(previous);
                    }
                }
                if let Some(current) = fg_name.clone() {
                    if security.protected_apps.contains(&current) && armed.remove(&current) {
                        runtime.locked = true;
                        runtime.locked_app = Some(current.clone());
                        let _ = app.emit("guard-lock", &current);
                    }
                }
            }
            *last = fg_name;
        }

        let sleep_ms = if settings.aggressive_mode { 700 } else { 1200 };
        std::thread::sleep(Duration::from_millis(sleep_ms));
    });
}

#[tauri::command]
fn get_system_snapshot(state: State<'_, Arc<SharedState>>) -> SystemSnapshot {
    let optimized = state.optimized.lock().unwrap();
    collect_snapshot(&optimized)
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
        return Ok(vec![]);
    }
    #[cfg(windows)]
    {
        let script = r#"
$items = @()
function Add-Item($source, $name, $path, $type, $enabled, $details) {
  $items += [PSCustomObject]@{
    id = "$source::$name::$type"
    source = $source
    name = $name
    path = $path
    itemType = $type
    enabled = $enabled
    details = $details
  }
}
$regTargets = @(
  @{ Hive = 'HKCU'; Key = 'Software\Microsoft\Windows\CurrentVersion\Run'; Label='HKCU Run' },
  @{ Hive = 'HKLM'; Key = 'Software\Microsoft\Windows\CurrentVersion\Run'; Label='HKLM Run' },
  @{ Hive = 'HKCU'; Key = 'Software\Microsoft\Windows\CurrentVersion\RunOnce'; Label='HKCU RunOnce' },
  @{ Hive = 'HKLM'; Key = 'Software\Microsoft\Windows\CurrentVersion\RunOnce'; Label='HKLM RunOnce' }
)
foreach ($target in $regTargets) {
  try {
    $full = "$($target.Hive):\$($target.Key)"
    $props = Get-ItemProperty -Path $full
    foreach ($p in $props.PSObject.Properties) {
      if ($p.Name -notmatch '^PS') { Add-Item $target.Label $p.Name ([string]$p.Value) 'Registry' $true $full }
    }
  } catch {}
}
$startupPaths = @(
  @{ Label='Startup User'; Path=[Environment]::GetFolderPath('Startup') },
  @{ Label='Startup Common'; Path=$env:ProgramData + '\Microsoft\Windows\Start Menu\Programs\Startup' }
)
foreach ($target in $startupPaths) {
  if (Test-Path $target.Path) {
    Get-ChildItem -Path $target.Path -Force | ForEach-Object {
      Add-Item $target.Label $_.Name $_.FullName 'StartupFolder' ($_.Extension -ne '.disabled') $target.Path
    }
  }
}
try {
  Get-ScheduledTask | Where-Object { $_.TaskPath -notlike '\Microsoft*' } | ForEach-Object {
    $task = $_
    $action = ($task.Actions | Select-Object -First 1)
    Add-Item 'Scheduled Task' $task.TaskName ($action.Execute + ' ' + $action.Arguments) 'Task' ($task.State -ne 'Disabled') $task.TaskPath
  }
} catch {}
try {
  Get-CimInstance Win32_Service | Where-Object { $_.StartMode -in @('Auto','Automatic') } | ForEach-Object {
    Add-Item 'Service' $_.Name $_.PathName 'Service' ($_.StartMode -ne 'Disabled') $_.DisplayName
  }
} catch {}
$items | ConvertTo-Json -Depth 4
"#;
        let out = powershell(script)?;
        serde_json::from_str(&out).or_else(|_| Ok(vec![]))
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
fn reg_u32(path: &str, name: &str) -> Option<u32> {
    use winreg::{enums::*, RegKey};
    let path = path.trim_start_matches("HKLM\\");
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey(path).ok()?.get_value::<u32, _>(name).ok()
}

#[cfg(not(windows))]
fn reg_u32(_path: &str, _name: &str) -> Option<u32> { None }

#[tauri::command]
fn get_registry_health() -> Vec<RegistryHealthItem> {
    let checks = vec![
        ("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System", "EnableLUA", 1, "UAC bazowo włączony"),
        ("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System", "PromptOnSecureDesktop", 1, "Monit UAC na bezpiecznym pulpicie"),
        ("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System", "ConsentPromptBehaviorAdmin", 5, "Rozsądny prompt dla admina"),
        ("HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa", "RunAsPPL", 1, "LSA Protection"),
        ("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer", "SmartScreenEnabled", 1, "SmartScreen / reputacja plików"),
    ];
    checks
        .into_iter()
        .map(|(path, value, recommended, meaning)| {
            let current = reg_u32(path, value)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "brak / n.d.".to_string());
            RegistryHealthItem {
                key_path: path.to_string(),
                value_name: value.to_string(),
                current: current.clone(),
                recommended: recommended.to_string(),
                healthy: current == recommended.to_string(),
                meaning: meaning.to_string(),
            }
        })
        .collect()
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

#[tauri::command]
fn get_security_config(state: State<'_, Arc<SharedState>>) -> SecurityConfigView {
    let config = state.security_config.lock().unwrap().clone();
    SecurityConfigView {
        password_set: config.password_hash.is_some(),
        file_protection: config.file_protection,
        protected_apps: config.protected_apps,
        lock_enabled: config.lock_enabled,
    }
}

#[tauri::command]
fn save_security_config(payload: SecurityConfigPayload, state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    let mut config = state.security_config.lock().unwrap();
    if let Some(password) = payload.password {
        if !password.trim().is_empty() {
            config.password_hash = Some(hash_password(&password));
        }
    }
    config.file_protection = payload.file_protection;
    config.protected_apps = payload
        .protected_apps
        .into_iter()
        .map(|x| x.to_lowercase())
        .collect();
    config.lock_enabled = payload.lock_enabled;
    save_json(&security_path(), &*config)
}

#[tauri::command]
fn get_security_runtime(state: State<'_, Arc<SharedState>>) -> SecurityRuntime {
    state.security_runtime.lock().unwrap().clone()
}

#[tauri::command]
fn unlock_guard(password: String, state: State<'_, Arc<SharedState>>) -> bool {
    let config = state.security_config.lock().unwrap().clone();
    if config.password_hash == Some(hash_password(&password)) {
        let mut runtime = state.security_runtime.lock().unwrap();
        runtime.locked = false;
        runtime.locked_app = None;
        true
    } else {
        false
    }
}

#[tauri::command]
fn get_settings(state: State<'_, Arc<SharedState>>) -> SettingsState {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(settings: SettingsState, state: State<'_, Arc<SharedState>>) -> Result<(), String> {
    *state.settings.lock().unwrap() = settings.clone();
    save_json(&settings_path(), &settings)
}

fn main() {
    let shared = Arc::new(SharedState::default());
    *shared.rules.lock().unwrap() = load_json::<Vec<OptimizationRule>>(&rules_path());
    *shared.security_config.lock().unwrap() = load_json::<SecurityConfig>(&security_path());
    *shared.settings.lock().unwrap() = load_json::<SettingsState>(&settings_path());

    tauri::Builder::default()
        .manage(shared.clone())
        .setup(move |app| {
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
            get_security_config,
            save_security_config,
            get_security_runtime,
            unlock_guard,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
