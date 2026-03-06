use winreg::{enums::*, RegKey};

use crate::models::RegistryCheck;

fn read_u32(root: HKEY, path: &str, value_name: &str) -> String {
    let hk = RegKey::predef(root);
    hk.open_subkey(path)
        .ok()
        .and_then(|sub| sub.get_value::<u32, _>(value_name).ok().map(|v| v.to_string()))
        .unwrap_or_else(|| "missing".to_string())
}

fn read_string(root: HKEY, path: &str, value_name: &str) -> String {
    let hk = RegKey::predef(root);
    hk.open_subkey(path)
        .ok()
        .and_then(|sub| sub.get_value::<String, _>(value_name).ok())
        .unwrap_or_else(|| "missing".to_string())
}

pub fn get_registry_checks() -> Vec<RegistryCheck> {
    let checks = vec![
        RegistryCheck {
            id: "uac-enable".into(),
            label: "UAC / EnableLUA".into(),
            path: r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System".into(),
            value_name: "EnableLUA".into(),
            current_value: read_u32(
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
                "EnableLUA",
            ),
            recommended_value: "1".into(),
            status: "ok".into(),
            severity: "high".into(),
            description: "User Account Control powinien być włączony.".into(),
            can_fix: true,
        },
        RegistryCheck {
            id: "uac-secure-desktop".into(),
            label: "UAC / Secure Desktop".into(),
            path: r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System".into(),
            value_name: "PromptOnSecureDesktop".into(),
            current_value: read_u32(
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
                "PromptOnSecureDesktop",
            ),
            recommended_value: "1".into(),
            status: "ok".into(),
            severity: "medium".into(),
            description: "Monit UAC powinien przełączać na bezpieczny pulpit.".into(),
            can_fix: true,
        },
        RegistryCheck {
            id: "smart-screen".into(),
            label: "Microsoft Defender SmartScreen".into(),
            path: r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer".into(),
            value_name: "SmartScreenEnabled".into(),
            current_value: read_string(
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer",
                "SmartScreenEnabled",
            ),
            recommended_value: "Warn".into(),
            status: "ok".into(),
            severity: "medium".into(),
            description: "SmartScreen pomaga blokować nieznane i ryzykowne binarki.".into(),
            can_fix: true,
        },
        RegistryCheck {
            id: "lsa-ppl".into(),
            label: "LSA Protected Process".into(),
            path: r"HKLM\SYSTEM\CurrentControlSet\Control\Lsa".into(),
            value_name: "RunAsPPL".into(),
            current_value: read_u32(HKEY_LOCAL_MACHINE, r"SYSTEM\CurrentControlSet\Control\Lsa", "RunAsPPL"),
            recommended_value: "1".into(),
            status: "ok".into(),
            severity: "high".into(),
            description: "Chroni Local Security Authority przed częścią ataków pamięciowych.".into(),
            can_fix: true,
        },
        RegistryCheck {
            id: "remote-uac-filter".into(),
            label: "Remote UAC token filtering".into(),
            path: r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System".into(),
            value_name: "LocalAccountTokenFilterPolicy".into(),
            current_value: read_u32(
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
                "LocalAccountTokenFilterPolicy",
            ),
            recommended_value: "0 / missing".into(),
            status: "ok".into(),
            severity: "medium".into(),
            description: "Preferowane jest brak wyłączonego filtrowania tokena dla zdalnych logowań.".into(),
            can_fix: true,
        },
    ];

    checks
        .into_iter()
        .map(|mut check| {
            check.status = match (check.id.as_str(), check.current_value.as_str(), check.recommended_value.as_str()) {
                ("smart-screen", "Warn", _) => "ok".into(),
                ("remote-uac-filter", "missing", _) | ("remote-uac-filter", "0", _) => "ok".into(),
                (_, current, recommended) if current == recommended => "ok".into(),
                (_, "missing", _) => "warn".into(),
                (_, _, _) if check.severity == "high" => "critical".into(),
                _ => "warn".into(),
            };
            check
        })
        .collect()
}

pub fn fix_registry_check(check_id: &str) -> anyhow::Result<Vec<RegistryCheck>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (path, _disp) = hklm.create_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System")?;
    let (explorer_path, _disp) = hklm.create_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer")?;
    let (lsa_path, _disp) = hklm.create_subkey(r"SYSTEM\CurrentControlSet\Control\Lsa")?;

    match check_id {
        "uac-enable" => path.set_value("EnableLUA", &1u32)?,
        "uac-secure-desktop" => path.set_value("PromptOnSecureDesktop", &1u32)?,
        "smart-screen" => explorer_path.set_value("SmartScreenEnabled", &"Warn")?,
        "lsa-ppl" => lsa_path.set_value("RunAsPPL", &1u32)?,
        "remote-uac-filter" => {
            let _ = path.delete_value("LocalAccountTokenFilterPolicy");
        }
        _ => {}
    }

    Ok(get_registry_checks())
}
