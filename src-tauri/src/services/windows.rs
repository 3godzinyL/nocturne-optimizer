#[cfg(target_os = "windows")]
mod imp {
    use std::{collections::HashMap, mem::size_of};

    use windows::{
        core::PCSTR,
        Win32::{
            Foundation::{BOOL, HANDLE, HWND, LPARAM, RECT},
            System::{
                Memory::EmptyWorkingSet,
                Threading::{
                    OpenProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS,
                    IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_QUERY_INFORMATION,
                    PROCESS_SET_INFORMATION, PROCESS_SUSPEND_RESUME, PROCESS_VM_OPERATION,
                    PROCESS_VM_READ,
                },
            },
            UI::WindowsAndMessaging::{
                EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowTextLengthW,
                GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
            },
        },
    };

    #[derive(Debug, Clone, Default)]
    pub struct WindowInfo {
        pub visible: bool,
        pub minimized: bool,
        pub title: String,
        pub rect: Option<(i32, i32, i32, i32)>,
    }

    #[derive(Debug, Clone)]
    pub struct OptimizationFlags {
        pub lower_priority: bool,
        pub trim_memory: bool,
        pub suspend: bool,
        pub aggressive_idle: bool,
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtSuspendProcess(process_handle: HANDLE) -> i32;
        fn NtResumeProcess(process_handle: HANDLE) -> i32;
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let map = &mut *(lparam.0 as *mut HashMap<u32, WindowInfo>);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return true.into();
        }

        let visible = IsWindowVisible(hwnd).as_bool();
        let minimized = IsIconic(hwnd).as_bool();

        let title_len = GetWindowTextLengthW(hwnd);
        let mut title_buf = vec![0u16; (title_len + 1) as usize];
        let read_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..read_len as usize]);

        let mut rect = RECT::default();
        let rect_result = GetWindowRect(hwnd, &mut rect);

        let entry = map.entry(pid).or_default();
        entry.visible |= visible;
        entry.minimized |= minimized;

        if entry.title.is_empty() && !title.trim().is_empty() {
            entry.title = title;
        }

        if rect_result.as_bool() {
            entry.rect = Some((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top));
        }

        true.into()
    }

    pub fn foreground_pid() -> u32 {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return 0;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            pid
        }
    }

    pub fn enumerate_windows() -> HashMap<u32, WindowInfo> {
        let mut map = HashMap::<u32, WindowInfo>::new();
        unsafe {
            let _ = EnumWindows(Some(enum_windows_proc), LPARAM((&mut map as *mut _) as isize));
        }
        map
    }

    pub fn apply_optimization(pid: u32, flags: &OptimizationFlags) -> Result<(), String> {
        unsafe {
            let handle = OpenProcess(
                PROCESS_QUERY_INFORMATION
                    | PROCESS_SET_INFORMATION
                    | PROCESS_VM_READ
                    | PROCESS_VM_OPERATION
                    | PROCESS_SUSPEND_RESUME,
                false,
                pid,
            )
            .map_err(|e| e.to_string())?;

            if flags.lower_priority {
                let priority = if flags.aggressive_idle {
                    IDLE_PRIORITY_CLASS
                } else {
                    BELOW_NORMAL_PRIORITY_CLASS
                };
                let _ = SetPriorityClass(handle, priority);
            }

            if flags.trim_memory {
                let _ = EmptyWorkingSet(handle);
            }

            if flags.suspend {
                let status = NtSuspendProcess(handle);
                if status != 0 {
                    return Err(format!("NtSuspendProcess failed with status {}", status));
                }
            }

            Ok(())
        }
    }

    pub fn restore_process(pid: u32) -> Result<(), String> {
        unsafe {
            let handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION | PROCESS_SUSPEND_RESUME,
                false,
                pid,
            )
            .map_err(|e| e.to_string())?;

            let _ = SetPriorityClass(handle, NORMAL_PRIORITY_CLASS);
            let status = NtResumeProcess(handle);
            if status != 0 {
                // Treat this as non-fatal because process may not currently be suspended.
            }
            Ok(())
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use std::collections::HashMap;

    #[derive(Debug, Clone, Default)]
    pub struct WindowInfo {
        pub visible: bool,
        pub minimized: bool,
        pub title: String,
        pub rect: Option<(i32, i32, i32, i32)>,
    }

    #[derive(Debug, Clone)]
    pub struct OptimizationFlags {
        pub lower_priority: bool,
        pub trim_memory: bool,
        pub suspend: bool,
        pub aggressive_idle: bool,
    }

    pub fn foreground_pid() -> u32 {
        0
    }

    pub fn enumerate_windows() -> HashMap<u32, WindowInfo> {
        HashMap::new()
    }

    pub fn apply_optimization(_pid: u32, _flags: &OptimizationFlags) -> Result<(), String> {
        Ok(())
    }

    pub fn restore_process(_pid: u32) -> Result<(), String> {
        Ok(())
    }
}

pub use imp::*;
