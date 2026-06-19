use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Manager;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTONEAREST, MONITORINFO,
};
#[cfg(target_os = "windows")]
use windows::core::PWSTR;
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible,
};

pub struct AttachState {
    pub running: Arc<Mutex<bool>>,
    pub card_width: Arc<Mutex<i32>>,
    pub collapsed_height: Arc<Mutex<HashMap<String, i32>>>,
    pub attach_enabled: Arc<Mutex<HashMap<String, bool>>>,
    pub attach_whitelist: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl AttachState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            card_width: Arc::new(Mutex::new(360)),
            collapsed_height: Arc::new(Mutex::new(HashMap::new())),
            attach_enabled: Arc::new(Mutex::new(HashMap::new())),
            attach_whitelist: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
}

#[cfg(target_os = "windows")]
fn get_window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len as usize])
}

#[cfg(target_os = "windows")]
fn get_process_name(pid: u32) -> String {
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
            let mut buf = [0u16; 512];
            let mut size = buf.len() as u32;
            if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size).is_ok() {
                let full = String::from_utf16_lossy(&buf[..size as usize]);
                // Extract just the filename
                return full.rsplit('\\').next().unwrap_or(&full).to_string();
            }
        }
    }
    String::new()
}

/// Check if a process name matches any entry in the whitelist.
/// Empty whitelist = no match (hide the widget).
fn matches_whitelist(process: &str, whitelist: &[String]) -> bool {
    if whitelist.is_empty() {
        return false;
    }
    let proc_lower = process.to_lowercase();
    whitelist.iter().any(|pattern| {
        proc_lower.contains(&pattern.to_lowercase())
    })
}

/// Get the process name of a window by its PID.
#[cfg(target_os = "windows")]
fn get_fg_process_name(hwnd: HWND) -> String {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)); }
    get_process_name(pid)
}

#[cfg(target_os = "windows")]
fn is_own_window(hwnd: HWND) -> bool {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    let my_pid = unsafe { GetCurrentProcessId() };
    pid == my_pid
}

const WIDGET_LABELS: &[&str] = &["widget-git", "widget-amkr"];

/// Entry returned by `list_visible_windows`.
#[derive(serde::Serialize, Clone)]
pub struct WindowEntry {
    /// Process executable name (e.g. "Code.exe")
    pub process: String,
    /// Example window title from this process
    pub title: String,
}

/// List all visible top-level windows with non-empty titles.
/// Used by the frontend to let the user pick whitelist entries.
#[cfg(target_os = "windows")]
#[tauri::command]
pub fn list_visible_windows() -> Vec<WindowEntry> {
    let entries = Arc::new(Mutex::new(Vec::<WindowEntry>::new()));
    let entries_clone = entries.clone();

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: windows::Win32::Foundation::LPARAM) -> BOOL {
        let entries = unsafe { &*(lparam.0 as *const Mutex<Vec<WindowEntry>>) };
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let title = get_window_title(hwnd);
            if !title.is_empty() {
                let mut pid: u32 = 0;
                unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)); }
                let my_pid = unsafe { GetCurrentProcessId() };
                if pid != my_pid {
                    let process = get_process_name(pid);
                    entries.lock().unwrap().push(WindowEntry { title, process });
                }
            }
        }
        BOOL(1) // continue enumeration
    }

    unsafe {
        let _ = EnumWindows(Some(enum_cb), windows::Win32::Foundation::LPARAM(
            &*entries_clone as *const Mutex<Vec<WindowEntry>> as isize,
        ));
    }

    let result = entries.lock().unwrap().clone();
    // Deduplicate by process name, keep first window title as example
    let mut seen = std::collections::HashSet::new();
    result.into_iter().filter(|e| seen.insert(e.process.clone())).collect()
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn list_visible_windows() -> Vec<WindowEntry> {
    Vec::new()
}

#[cfg(target_os = "windows")]
pub fn start_attach_loop(app_handle: tauri::AppHandle, state: Arc<AttachState>) {
    let running = state.running.clone();
    let card_width = state.card_width.clone();
    let collapsed_height = state.collapsed_height.clone();
    let attach_enabled = state.attach_enabled.clone();
    let attach_whitelist = state.attach_whitelist.clone();

    {
        let mut r = running.lock().unwrap();
        *r = true;
    }

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));

        loop {
            {
                let r = running.lock().unwrap();
                if !*r {
                    break;
                }
            }

            let cw = *card_width.lock().unwrap();
            let collapsed_map = collapsed_height.lock().unwrap().clone();
            let enabled_map = attach_enabled.lock().unwrap().clone();
            let whitelist_map = attach_whitelist.lock().unwrap().clone();

            // Handle collapsed windows
            for &label in WIDGET_LABELS {
                if let Some(ch) = collapsed_map.get(label) {
                    if let Some(win) = app_handle.get_webview_window(label) {
                        let _ = win.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                            width: cw as u32,
                            height: *ch as u32,
                        }));
                    }
                }
            }

            let fg = unsafe { GetForegroundWindow() };

            if is_own_window(fg) {
                thread::sleep(Duration::from_millis(250));
                continue;
            }

            let fg_process = get_fg_process_name(fg);

            // Hide widgets whose whitelist doesn't match the foreground process
            for &label in WIDGET_LABELS {
                if collapsed_map.contains_key(label) {
                    continue;
                }
                if enabled_map.get(label).copied() == Some(false) {
                    continue;
                }
                let wl = whitelist_map.get(label).map(|v| v.as_slice()).unwrap_or(&[]);
                if !matches_whitelist(&fg_process, wl) {
                    if let Some(win) = app_handle.get_webview_window(label) {
                        let _ = win.hide();
                    }
                }
            }

            // Check if any widget should be visible
            let any_visible = WIDGET_LABELS.iter().any(|&label| {
                if collapsed_map.contains_key(label) { return false; }
                if enabled_map.get(label).copied() == Some(false) { return false; }
                let wl = whitelist_map.get(label).map(|v| v.as_slice()).unwrap_or(&[]);
                matches_whitelist(&fg_process, wl)
            });

            if !any_visible {
                thread::sleep(Duration::from_millis(250));
                continue;
            }

            // Position matching widgets
            let mut target_rect = RECT::default();
            let got_rect = unsafe { GetWindowRect(fg, &mut target_rect) };

            if got_rect.is_ok() {
                let rect = Rect {
                    left: target_rect.left,
                    top: target_rect.top,
                    right: target_rect.right,
                };

                let hmonitor = unsafe { MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST) };
                let mut mi = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                let got_monitor = unsafe { GetMonitorInfoW(hmonitor, &mut mi) };

                if got_monitor.as_bool() {
                    let work_left = mi.rcWork.left;
                    let work_right = mi.rcWork.right;
                    let work_top = mi.rcWork.top;

                    let mut y_offset: i32 = 0;
                    for &label in WIDGET_LABELS {
                        if collapsed_map.contains_key(label) { continue; }
                        if enabled_map.get(label).copied() == Some(false) { continue; }
                        let wl = whitelist_map.get(label).map(|v| v.as_slice()).unwrap_or(&[]);
                        if !matches_whitelist(&fg_process, wl) { continue; }

                        if let Some(win) = app_handle.get_webview_window(label) {
                            let _ = win.show();

                            let card_x = if rect.right + cw <= work_right {
                                rect.right
                            } else if rect.left - cw >= work_left {
                                rect.left - cw
                            } else {
                                work_right - cw
                            };

                            let card_y = (rect.top + y_offset).max(work_top);

                            let _ = win.set_position(tauri::Position::Physical(
                                tauri::PhysicalPosition { x: card_x, y: card_y },
                            ));

                            if let Ok(phys_size) = win.outer_size() {
                                y_offset += phys_size.height as i32;
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(250));
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start_attach_loop(_app_handle: tauri::AppHandle, _state: Arc<AttachState>) {}

#[allow(dead_code)]
pub fn stop_attach_loop(state: &AttachState) {
    let mut r = state.running.lock().unwrap();
    *r = false;
}
