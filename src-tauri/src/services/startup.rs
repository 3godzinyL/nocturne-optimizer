use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Result};
use serde_json::Value;
use winreg::{enums::*, RegKey};

use crate::models::AutostartEntry;

fn registry_sources() -> Vec<(HKEY, &'static str, &'static str)> {
    vec![
        (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Run", "HKCU Run"),
        (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\RunOnce", "HKCU RunOnce"),
        (HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Run", "HKLM Run"),
        (HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\RunOnce", "HKLM RunOnce"),
        (HKEY_LOCAL_MACHINE, r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run", "HKLM Wow6432 Run"),
    ]
}

fn registry_entries() -> Vec<AutostartEntry> {
    let mut items = vec![];

    for (root, path, source) in registry_sources() {
        let hk = RegKey::predef(root);
        if let Ok(sub) = hk.open_subkey(path) {
            for value in sub.enum_values().flatten() {
                let (name, data) = value;
                let command = String::from_utf8_lossy(&data.bytes).trim_matches(char::from(0)).to_string();
                let enabled = !name.starts_with("__disabled__nocturne__");
                items.push(AutostartEntry {
                    id: format!("reg|{}|{}|{}", root, path, name),
                    source: source.into(),
                    name: name.replace("__disabled__nocturne__", ""),
                    command,
                    location: format!(r"{}\{}", if root == HKEY_CURRENT_USER { "HKCU" } else { "HKLM" }, path),
                    enabled,
                    can_toggle: true,
                    kind: "Registry".into(),
                });
            }
        }
    }

    items
}

fn startup_folders() -> Vec<(String, PathBuf)> {
    let mut folders = vec![];

    if let Ok(appdata) = env::var("APPDATA") {
        folders.push((
            "Per-user Startup".into(),
            Path::new(&appdata)
                .join(r"Microsoft\Windows\Start Menu\Programs\Startup"),
        ));
    }

    if let Ok(program_data) = env::var("ProgramData") {
        folders.push((
            "Common Startup".into(),
            Path::new(&program_data)
                .join(r"Microsoft\Windows\Start Menu\Programs\Startup"),
        ));
    }

    folders
}

fn folder_entries() -> Vec<AutostartEntry> {
    let mut items = vec![];
    for (source, folder) in startup_folders() {
        if let Ok(entries) = fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let enabled = !name.ends_with(".disabled_by_nocturne");
                items.push(AutostartEntry {
                    id: format!("folder|{}", path.display()),
                    source: source.clone(),
                    name: name.replace(".disabled_by_nocturne", ""),
                    command: path.display().to_string(),
                    location: folder.display().to_string(),
                    enabled,
                    can_toggle: true,
                    kind: "Startup folder".into(),
                });
            }
        }
    }
    items
}

fn powershell_json(script: &str) -> Result<Value> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "powershell failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Value::Null);
    }

    let parsed = serde_json::from_str::<Value>(&stdout)?;
    Ok(parsed)
}

fn scheduled_task_entries() -> Vec<AutostartEntry> {
    let script = r#"Get-ScheduledTask | Where-Object {
        $_.Triggers | Where-Object {
            $_.CimClass.CimClassName -in @("MSFT_TaskLogonTrigger","MSFT_TaskBootTrigger")
        }
      } | Select-Object TaskName,TaskPath,State,@{N='Enabled';E={$_.Settings.Enabled}} | ConvertTo-Json -Depth 5"#;

    let value = powershell_json(script).unwrap_or(Value::Null);
    let mut items = vec![];

    let arr = match value {
        Value::Array(values) => values,
        Value::Null => vec![],
        single => vec![single],
    };

    for row in arr {
        let task_name = row.get("TaskName").and_then(Value::as_str).unwrap_or_default();
        let task_path = row.get("TaskPath").and_then(Value::as_str).unwrap_or("\\");
        let enabled = row.get("Enabled").and_then(Value::as_bool).unwrap_or(true);
        let state = row.get("State").and_then(Value::as_str).unwrap_or_default();

        items.push(AutostartEntry {
            id: format!("task|{}{}", task_path, task_name),
            source: "Scheduled Tasks".into(),
            name: task_name.into(),
            command: format!("{}{}", task_path, task_name),
            location: task_path.into(),
            enabled,
            can_toggle: true,
            kind: format!("Task ({state})"),
        });
    }

    items
}

fn service_entries() -> Vec<AutostartEntry> {
    let script = r#"Get-CimInstance Win32_Service | Where-Object {
        $_.StartMode -eq 'Auto'
      } | Select-Object Name,DisplayName,StartMode,State,PathName | ConvertTo-Json -Depth 4"#;

    let value = powershell_json(script).unwrap_or(Value::Null);
    let mut items = vec![];

    let arr = match value {
        Value::Array(values) => values,
        Value::Null => vec![],
        single => vec![single],
    };

    for row in arr {
        let service_name = row.get("Name").and_then(Value::as_str).unwrap_or_default();
        let display_name = row.get("DisplayName").and_then(Value::as_str).unwrap_or(service_name);
        let path_name = row.get("PathName").and_then(Value::as_str).unwrap_or_default();
        let state = row.get("State").and_then(Value::as_str).unwrap_or_default();

        items.push(AutostartEntry {
            id: format!("service|{}", service_name),
            source: "Services".into(),
            name: display_name.into(),
            command: path_name.into(),
            location: service_name.into(),
            enabled: true,
            can_toggle: true,
            kind: format!("Service ({state})"),
        });
    }

    items
}

pub fn collect_all() -> Vec<AutostartEntry> {
    let mut items = vec![];
    items.extend(registry_entries());
    items.extend(folder_entries());
    items.extend(scheduled_task_entries());
    items.extend(service_entries());

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}

pub fn toggle(entry_id: &str, enabled: bool) -> Result<Vec<AutostartEntry>> {
    let parts: Vec<&str> = entry_id.split('|').collect();

    match parts.first().copied().unwrap_or_default() {
        "reg" => {
            let root = if parts.get(1).copied() == Some("2147483649") {
                HKEY_CURRENT_USER
            } else {
                HKEY_LOCAL_MACHINE
            };
            let path = parts.get(2).copied().unwrap_or_default();
            let name = parts.get(3).copied().unwrap_or_default();

            let hk = RegKey::predef(root);
            let key = hk.open_subkey_with_flags(path, KEY_READ | KEY_WRITE)?;
            let value_name = if enabled {
                name.replacen("__disabled__nocturne__", "", 1)
            } else if name.starts_with("__disabled__nocturne__") {
                name.to_string()
            } else {
                format!("__disabled__nocturne__{}", name)
            };

            if enabled && name.starts_with("__disabled__nocturne__") {
                if let Ok(value) = key.get_raw_value(name) {
                    key.set_raw_value(value_name, &value)?;
                    let _ = key.delete_value(name);
                }
            } else if !enabled && !name.starts_with("__disabled__nocturne__") {
                if let Ok(value) = key.get_raw_value(name) {
                    key.set_raw_value(value_name, &value)?;
                    let _ = key.delete_value(name);
                }
            }
        }
        "folder" => {
            let full_path = parts.get(1).copied().unwrap_or_default();
            let path = PathBuf::from(full_path);
            if enabled {
                if full_path.ends_with(".disabled_by_nocturne") {
                    let restored = full_path.trim_end_matches(".disabled_by_nocturne");
                    let _ = fs::rename(&path, restored);
                }
            } else if path.exists() {
                let _ = fs::rename(&path, format!("{}.disabled_by_nocturne", full_path));
            }
        }
        "task" => {
            let task_name = parts.get(1).copied().unwrap_or_default();
            let action = if enabled { "/Enable" } else { "/Disable" };
            let _ = Command::new("schtasks")
                .args(["/Change", "/TN", task_name, action])
                .output()?;
        }
        "service" => {
            let service_name = parts.get(1).copied().unwrap_or_default();
            let start_mode = if enabled { "auto" } else { "demand" };
            let _ = Command::new("sc.exe")
                .args(["config", service_name, "start=", start_mode])
                .output()?;
        }
        _ => {}
    }

    Ok(collect_all())
}
