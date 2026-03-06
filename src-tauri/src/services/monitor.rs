use std::{sync::Arc, thread, time::Duration};

use chrono::Local;
use sysinfo::{ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Size};

use crate::{
    models::{
        DashboardSnapshot, OptimizationLevel, OverlayTarget, ProcessEntry, ProtectedAppCandidate,
    },
    state::{AppliedOptimization, RuntimeState},
};

use super::windows::{apply_optimization, enumerate_windows, foreground_pid, restore_process, OptimizationFlags};

fn bytes_to_gb(value: u64) -> f64 {
    value as f64 / 1024f64 / 1024f64 / 1024f64
}

fn should_exclude(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "system",
        "registry",
        "idle",
        "memory compression",
        "secure system",
        "service host",
        "fontdrvhost",
        "sihost",
        "dwm",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn process_name_matches(process_name: &str, rule_match: &str) -> bool {
    process_name
        .to_ascii_lowercase()
        .contains(&rule_match.to_ascii_lowercase())
}

fn build_flags(level: &OptimizationLevel, trim_memory: bool, lower_priority: bool, suspend_on_hide: bool) -> OptimizationFlags {
    OptimizationFlags {
        lower_priority,
        trim_memory,
        suspend: suspend_on_hide || matches!(level, OptimizationLevel::Freeze),
        aggressive_idle: matches!(level, OptimizationLevel::Freeze),
    }
}

pub fn refresh_process_snapshot(state: &Arc<RuntimeState>) -> (DashboardSnapshot, Vec<ProcessEntry>) {
    let mut system = System::new_all();
    system.refresh_memory();
    system.refresh_cpu_usage();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let windows = enumerate_windows();
    let fg_pid = foreground_pid();

    let rules = {
        let inner = state.inner.lock();
        inner.rules.clone()
    };

    let mut processes: Vec<ProcessEntry> = system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let pid = pid.as_u32();
            let name = process.name().to_string_lossy().to_string();
            let exe = process
                .exe()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let win_info = windows.get(&pid).cloned().unwrap_or_default();

            ProcessEntry {
                pid,
                name,
                exe,
                cpu_usage: process.cpu_usage(),
                memory_mb: process.memory() as f64 / 1024f64 / 1024f64,
                status: "idle".to_string(),
                can_optimize: !should_exclude(&process.name().to_string_lossy()),
                is_foreground: fg_pid == pid,
                is_window_visible: win_info.visible,
                is_minimized: win_info.minimized,
                rule_applied: None,
                optimization_level: None,
                window_title: win_info.title,
            }
        })
        .collect();

    processes.sort_by(|a, b| {
        b.cpu_usage
            .partial_cmp(&a.cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.memory_mb.partial_cmp(&a.memory_mb).unwrap_or(std::cmp::Ordering::Equal))
    });

    {
        let mut inner = state.inner.lock();

        if inner.settings.auto_apply_rules {
            for process in processes.iter_mut() {
                let matched_rule = rules
                    .iter()
                    .find(|rule| rule.enabled && !rule.process_match.trim().is_empty() && process_name_matches(&process.name, &rule.process_match));

                let win_info = windows.get(&process.pid).cloned().unwrap_or_default();

                if let Some(rule) = matched_rule {
                    let apply_due_to_focus = !rule.only_when_not_foreground || !process.is_foreground;
                    let apply_due_to_visibility = !rule.only_when_hidden || win_info.minimized || !win_info.visible;

                    if process.can_optimize && apply_due_to_focus && apply_due_to_visibility {
                        let flags = build_flags(
                            &rule.level,
                            rule.trim_memory,
                            rule.lower_priority,
                            rule.suspend_on_hide && (win_info.minimized || !win_info.visible),
                        );

                        let _ = apply_optimization(process.pid, &flags);
                        inner.optimized.insert(
                            process.pid,
                            AppliedOptimization {
                                rule_id: rule.id.clone(),
                                level: rule.level.clone(),
                            },
                        );

                        process.status = "optimized".to_string();
                        process.rule_applied = Some(rule.process_match.clone());
                        process.optimization_level = Some(rule.level.to_string());
                    } else if inner.optimized.remove(&process.pid).is_some() {
                        let _ = restore_process(process.pid);
                        process.status = "restored".into();
                    } else {
                        process.status = if process.is_foreground { "active" } else { "visible" }.into();
                    }
                } else {
                    if inner.optimized.remove(&process.pid).is_some() {
                        let _ = restore_process(process.pid);
                    }
                    process.status = if process.is_foreground {
                        "active"
                    } else if process.is_minimized {
                        "minimized"
                    } else if process.is_window_visible {
                        "visible"
                    } else {
                        "background"
                    }
                    .into();
                }
            }
        }

        for process in processes.iter_mut() {
            if let Some(applied) = inner.optimized.get(&process.pid) {
                process.rule_applied = process.rule_applied.clone().or(Some(applied.rule_id.clone()));
                process.optimization_level = Some(applied.level.to_string());
                process.status = if process.is_foreground {
                    "foreground-restored".into()
                } else {
                    "optimized".into()
                };
            }
        }

        inner.security.popular_candidates = super::security::detect_popular_candidates(&processes);
        inner.processes = processes.clone();
    }

    let active_process_name = processes
        .iter()
        .find(|p| p.is_foreground)
        .map(|p| p.name.clone())
        .unwrap_or_default();

    let dashboard = DashboardSnapshot {
        cpu_usage: system.global_cpu_usage(),
        memory_used_gb: bytes_to_gb(system.used_memory()),
        memory_total_gb: bytes_to_gb(system.total_memory()),
        memory_used_pct: if system.total_memory() > 0 {
            (system.used_memory() as f32 / system.total_memory() as f32) * 100.0
        } else {
            0.0
        },
        swap_used_gb: bytes_to_gb(system.used_swap()),
        swap_total_gb: bytes_to_gb(system.total_swap()),
        uptime_secs: System::uptime(),
        refreshed_at: Local::now().format("%H:%M:%S").to_string(),
        network_rx_mb: 0.0,
        network_tx_mb: 0.0,
        active_process_name,
        top_processes: processes.iter().take(8).cloned().collect(),
    };

    {
        let mut inner = state.inner.lock();
        inner.dashboard = dashboard.clone();
    }

    (dashboard, processes)
}

fn maybe_show_overlay(app: &AppHandle, state: &Arc<RuntimeState>) {
    let windows = enumerate_windows();
    let foreground = foreground_pid();

    let candidate = {
        let mut inner = state.inner.lock();
        let security = inner.security.clone();
        let protected_names: Vec<String> = security
            .protected_targets
            .iter()
            .filter(|t| t.enabled)
            .map(|t| t.process_name.to_ascii_lowercase())
            .collect();

        if protected_names.is_empty() || !security.password_enabled {
            inner.security.overlay_active = false;
            inner.overlay_target = None;
            inner.security.overlay_target = None;
            return;
        }

        let processes = inner.processes.clone();

        for process in processes.iter() {
            let monitored = protected_names
                .iter()
                .any(|name| process.name.to_ascii_lowercase() == *name);

            if !monitored {
                continue;
            }

            let was_minimized = inner.last_minimized.get(&process.pid).copied().unwrap_or(false);
            inner.last_minimized.insert(process.pid, process.is_minimized);

            if process.is_minimized {
                inner.locked_pids.insert(process.pid);
            }

            if !process.is_minimized
                && was_minimized
                && inner.locked_pids.contains(&process.pid)
                && foreground == process.pid
            {
                let win = windows.get(&process.pid).cloned().unwrap_or_default();
                if let Some((x, y, width, height)) = win.rect {
                    let overlay = OverlayTarget {
                        pid: process.pid,
                        process_name: process.name.clone(),
                        display_name: process.window_title.clone().if_empty_then(process.name.clone()),
                        x,
                        y,
                        width: width.max(420),
                        height: height.max(240),
                    };
                    inner.overlay_target = Some(overlay.clone());
                    inner.security.overlay_active = true;
                    inner.security.overlay_target = Some(overlay.clone());
                    return Some(overlay);
                }
            }
        }

        None
    };

    if let Some(overlay) = candidate {
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.set_size(Size::Logical(LogicalSize::new(
                overlay.width as f64,
                overlay.height as f64,
            )));
            let _ = window.set_position(LogicalPosition::new(overlay.x as f64, overlay.y as f64));
            let _ = app.emit("security://overlay_state", overlay.clone());
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

trait FallbackString {
    fn if_empty_then(self, fallback: String) -> String;
}

impl FallbackString for String {
    fn if_empty_then(self, fallback: String) -> String {
        if self.trim().is_empty() {
            fallback
        } else {
            self
        }
    }
}

pub fn start_monitor(app: AppHandle, state: Arc<RuntimeState>) {
    thread::spawn(move || loop {
        let refresh_ms = {
            let inner = state.inner.lock();
            inner.settings.refresh_interval_ms
        };

        let _ = refresh_process_snapshot(&state);
        maybe_show_overlay(&app, &state);
        thread::sleep(Duration::from_millis(refresh_ms.max(1000)));
    });
}
