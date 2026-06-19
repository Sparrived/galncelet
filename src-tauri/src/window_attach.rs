use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Manager;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTONEAREST, MONITORINFO,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, MSG, TranslateMessage, GetMessageW,
    EVENT_OBJECT_LOCATIONCHANGE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};

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
fn get_process_name(pid: u32) -> String {
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
            let mut buf = [0u16; 512];
            let mut size = buf.len() as u32;
            if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut size).is_ok() {
                let full = String::from_utf16_lossy(&buf[..size as usize]);
                return full.rsplit('\\').next().unwrap_or(&full).to_string();
            }
        }
    }
    String::new()
}

fn matches_whitelist(process: &str, whitelist: &[String]) -> bool {
    if whitelist.is_empty() { return false; }
    let p = process.to_lowercase();
    whitelist.iter().any(|w| p.contains(&w.to_lowercase()))
}

#[cfg(target_os = "windows")]
fn is_own_window(hwnd: HWND) -> bool {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)); }
    pid == unsafe { GetCurrentProcessId() }
}

const WIDGET_LABELS: &[&str] = &["widget-git", "widget-amkr"];

/// Reposition all enabled, non-collapsed widgets next to the given window.
#[cfg(target_os = "windows")]
fn reposition_widgets(app_handle: &tauri::AppHandle, target: HWND, state: &AttachState) {
    let cw = *state.card_width.lock().unwrap();
    let collapsed = state.collapsed_height.lock().unwrap().clone();
    let enabled = state.attach_enabled.lock().unwrap().clone();
    let whitelist = state.attach_whitelist.lock().unwrap().clone();

    // Get process name of target
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(target, Some(&mut pid)); }
    let process = get_process_name(pid);

    // Check which widgets should be visible
    let any_visible = WIDGET_LABELS.iter().any(|&label| {
        if collapsed.contains_key(label) { return false; }
        if enabled.get(label).copied() == Some(false) { return false; }
        let wl = whitelist.get(label).map(|v| v.as_slice()).unwrap_or(&[]);
        matches_whitelist(&process, wl)
    });

    if !any_visible {
        // Hide all non-collapsed widgets
        for &label in WIDGET_LABELS {
            if collapsed.contains_key(label) { continue; }
            if let Some(win) = app_handle.get_webview_window(label) {
                let _ = win.hide();
            }
        }
        return;
    }

    let mut target_rect = RECT::default();
    if unsafe { GetWindowRect(target, &mut target_rect) }.is_err() { return; }

    let hmonitor = unsafe { MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST) };
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(hmonitor, &mut mi) }.as_bool() { return; }

    let card_x = if target_rect.right + cw <= mi.rcWork.right {
        target_rect.right
    } else if target_rect.left - cw >= mi.rcWork.left {
        target_rect.left - cw
    } else {
        mi.rcWork.right - cw
    };
    let card_y = target_rect.top.max(mi.rcWork.top);

    let mut y_offset: i32 = 0;
    for &label in WIDGET_LABELS {
        if collapsed.contains_key(label) { continue; }
        if enabled.get(label).copied() == Some(false) { continue; }
        let wl = whitelist.get(label).map(|v| v.as_slice()).unwrap_or(&[]);
        if !matches_whitelist(&process, wl) {
            if let Some(win) = app_handle.get_webview_window(label) {
                let _ = win.hide();
            }
            continue;
        }

        if let Some(win) = app_handle.get_webview_window(label) {
            let _ = win.show();
            let _ = win.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition { x: card_x, y: card_y + y_offset },
            ));
            if let Ok(size) = win.outer_size() {
                y_offset += size.height as i32;
            }
        }
    }
}

// ─── list_visible_windows ───

#[derive(serde::Serialize, Clone)]
pub struct WindowEntry {
    pub process: String,
    pub title: String,
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn list_visible_windows() -> Vec<WindowEntry> {
    let entries = Arc::new(Mutex::new(Vec::<WindowEntry>::new()));
    let entries_clone = entries.clone();

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let entries = unsafe { &*(lparam.0 as *const Mutex<Vec<WindowEntry>>) };
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let mut buf = [0u16; 512];
            let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            if !title.is_empty() {
                let mut pid: u32 = 0;
                unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)); }
                if pid != unsafe { GetCurrentProcessId() } {
                    entries.lock().unwrap().push(WindowEntry { title, process: get_process_name(pid) });
                }
            }
        }
        BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(
            &*entries_clone as *const Mutex<Vec<WindowEntry>> as isize,
        ));
    }

    let result = entries.lock().unwrap().clone();
    let mut seen = std::collections::HashSet::new();
    result.into_iter().filter(|e| seen.insert(e.process.clone())).collect()
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn list_visible_windows() -> Vec<WindowEntry> { Vec::new() }

// ─── Attach: event-driven via SetWinEventHook ───

/// Flag set by the WinEventHook callback, read by the poll thread.
static WINDOW_DIRTY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub fn start_attach_loop(app_handle: tauri::AppHandle, state: Arc<AttachState>) {
    *state.running.lock().unwrap() = true;

    let state_hook = state.clone();

    // Thread 1: WinEventHook — fires instantly when any window moves
    thread::spawn(move || {
        unsafe extern "system" fn hook_callback(
            _hwin_event_hook: HWINEVENTHOOK,
            _event: u32,
            hwnd: HWND,
            _id_object: i32,
            _id_child: i32,
            _id_event_thread: u32,
            _dwms_event_time: u32,
        ) {
            let fg = unsafe { GetForegroundWindow() };
            if fg == hwnd && !is_own_window(fg) {
                WINDOW_DIRTY.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }

        unsafe {
            let _ = SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                None,
                Some(hook_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
        }

        // Pump messages to keep the hook alive
        let mut msg = MSG::default();
        loop {
            if !*state_hook.running.lock().unwrap() { break; }
            unsafe {
                let _ = GetMessageW(&mut msg, None, 0, 0);
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    // Thread 2: lightweight poll — repositions when dirty flag is set,
    // or when foreground window changes
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        let mut last_fg: isize = 0;

        loop {
            if !*state.running.lock().unwrap() { break; }

            let fg = unsafe { GetForegroundWindow() };
            let fg_changed = fg.0 as isize != last_fg;
            let dirty = WINDOW_DIRTY.load(std::sync::atomic::Ordering::Relaxed);

            if dirty || fg_changed {
                WINDOW_DIRTY.store(false, std::sync::atomic::Ordering::Relaxed);
                last_fg = fg.0 as isize;

                if !is_own_window(fg) {
                    reposition_widgets(&app_handle, fg, &state);
                }
            }

            thread::sleep(Duration::from_millis(50));
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start_attach_loop(_app_handle: tauri::AppHandle, _state: Arc<AttachState>) {}

#[allow(dead_code)]
pub fn stop_attach_loop(state: &AttachState) {
    *state.running.lock().unwrap() = false;
}
