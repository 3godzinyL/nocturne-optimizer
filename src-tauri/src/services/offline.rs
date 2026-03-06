use std::{
    fs,
    path::PathBuf,
    process::Command,
};

use crate::models::{OfflineProfile, OfflineProfileResult};

pub fn profiles() -> Vec<OfflineProfile> {
    vec![
        OfflineProfile {
            id: "clean-temp".into(),
            title: "Silent Clean".into(),
            description: "Czyści tymczasowe katalogi usera i śmieci po sesjach aplikacji bez ruszania kluczowych danych.".into(),
            actions: vec![
                "Usuń %TEMP% i typowe cache tymczasowe".into(),
                "Posprzątaj stare logi i pliki *.tmp".into(),
                "Opcjonalnie czyści Windows Temp, jeśli masz admina".into(),
            ],
            risk: "low".into(),
        },
        OfflineProfile {
            id: "debloat-lite".into(),
            title: "Windows Debloat Lite".into(),
            description: "Lekki profil uciszenia usług i tasków Xbox / Feedback / Consumer Experience, bez ciężkich ingerencji.".into(),
            actions: vec![
                "Ustaw wybrane usługi Xbox na manual".into(),
                "Wyłącz część tasków telemetry / customer experience".into(),
                "Odśwież wpisy autostartu po akcji".into(),
            ],
            risk: "medium".into(),
        },
        OfflineProfile {
            id: "offline-hard".into(),
            title: "Offline Hard Quiet".into(),
            description: "Mocniejszy preset pod maszynę, która ma być cicha w tle: usługi, taski i delivery optimization.".into(),
            actions: vec![
                "Wyłącz Delivery Optimization".into(),
                "Wyłącz dodatkowe taski CEIP i Appraiser".into(),
                "Wymaga admina i może wpłynąć na część funkcji Windows".into(),
            ],
            risk: "high".into(),
        },
    ]
}

fn clear_dir(path: PathBuf, details: &mut Vec<String>) {
    if !path.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&path) {
        for entry in entries.flatten() {
            let target = entry.path();
            let result = if target.is_dir() {
                fs::remove_dir_all(&target)
            } else {
                fs::remove_file(&target)
            };

            if result.is_ok() {
                details.push(format!("Removed {}", target.display()));
            }
        }
    }
}

fn run_cmd(program: &str, args: &[&str], details: &mut Vec<String>) -> String {
    let output = Command::new(program).args(args).output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            details.push(format!("{program} {:?} -> {}", args, output.status));
            format!("{}\n{}", stdout, stderr)
        }
        Err(err) => {
            details.push(format!("{program} {:?} -> failed: {}", args, err));
            err.to_string()
        }
    }
}

pub fn run_profile(profile_id: &str) -> OfflineProfileResult {
    let mut details = vec![];
    let mut output = String::new();

    match profile_id {
        "clean-temp" => {
            if let Ok(temp) = std::env::var("TEMP") {
                clear_dir(PathBuf::from(temp), &mut details);
            }
            clear_dir(PathBuf::from(r"C:\Windows\Temp"), &mut details);
            output.push_str("Temporary files cleanup completed.\n");
        }
        "debloat-lite" => {
            output.push_str(&run_cmd("sc.exe", &["config", "XblGameSave", "start=", "demand"], &mut details));
            output.push_str(&run_cmd("sc.exe", &["config", "XboxGipSvc", "start=", "demand"], &mut details));
            output.push_str(&run_cmd("schtasks", &["/Change", "/TN", r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser", "/Disable"], &mut details));
            output.push_str(&run_cmd("schtasks", &["/Change", "/TN", r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator", "/Disable"], &mut details));
        }
        "offline-hard" => {
            output.push_str(&run_cmd("reg", &["add", r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\DeliveryOptimization\Config", "/v", "DODownloadMode", "/t", "REG_DWORD", "/d", "0", "/f"], &mut details));
            output.push_str(&run_cmd("schtasks", &["/Change", "/TN", r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip", "/Disable"], &mut details));
            output.push_str(&run_cmd("schtasks", &["/Change", "/TN", r"\Microsoft\Windows\Autochk\Proxy", "/Disable"], &mut details));
            output.push_str(&run_cmd("sc.exe", &["config", "DiagTrack", "start=", "demand"], &mut details));
        }
        _ => {
            details.push("Unknown profile id".into());
            return OfflineProfileResult {
                profile_id: profile_id.to_string(),
                success: false,
                output: "Unknown profile".into(),
                details,
            };
        }
    }

    OfflineProfileResult {
        profile_id: profile_id.to_string(),
        success: true,
        output,
        details,
    }
}
