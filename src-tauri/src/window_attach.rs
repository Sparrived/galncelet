use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, Manager};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTONEAREST, MONITORINFO,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
#[cfg(target_os = "windows")]
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GetForegroundWindow, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, MSG, TranslateMessage, GetMessageW,
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
    /// When true for a widget, the attach system only manages show/hide,
    /// not position — the widget stays at its last manually-set position.
    pub attach_remember: Arc<Mutex<HashMap<String, bool>>>,
    /// Current browser URL (shared with frontend)
    pub current_url: Arc<Mutex<String>>,
    /// Widget labels that are part of the sequence — attach loop must not hide them.
    pub sequence_labels: Arc<Mutex<std::collections::HashSet<String>>>,
}


impl AttachState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            card_width: Arc::new(Mutex::new(360)),
            collapsed_height: Arc::new(Mutex::new(HashMap::new())),
            attach_enabled: Arc::new(Mutex::new(HashMap::new())),
            attach_whitelist: Arc::new(Mutex::new(HashMap::new())),
            attach_remember: Arc::new(Mutex::new(HashMap::new())),
            current_url: Arc::new(Mutex::new(String::new())),
            sequence_labels: Arc::new(Mutex::new(std::collections::HashSet::new())),
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
    if whitelist.is_empty() { return true; }
    let p = process.to_lowercase();
    whitelist.iter().any(|w| p.contains(&w.to_lowercase()))
}

#[cfg(target_os = "windows")]
fn is_own_window(hwnd: HWND) -> bool {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)); }
    pid == unsafe { GetCurrentProcessId() }
}

/// Cached state per widget to avoid redundant IPC calls.
struct WidgetCache {
    visible: bool,
    x: i32,
    y: i32,
}

/// Reposition all enabled, non-collapsed widgets next to the given window.
/// Widgets with attach_enabled = false are not affected by this logic.
/// Uses state caching to avoid redundant Tauri IPC calls.
/// `throttle_position`: when true (dirty-only, no fg change), skip set_position if called recently.
#[cfg(target_os = "windows")]
fn reposition_widgets(app_handle: &tauri::AppHandle, target: HWND, state: &AttachState, throttle_position: bool) {
    use std::collections::HashMap;
    use std::sync::Mutex;

    static WIDGET_CACHE: std::sync::OnceLock<Mutex<HashMap<String, WidgetCache>>> = std::sync::OnceLock::new();
    let cache = WIDGET_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    static LAST_POS_UPDATE: std::sync::OnceLock<Mutex<std::time::Instant>> = std::sync::OnceLock::new();
    let last_pos = LAST_POS_UPDATE.get_or_init(|| Mutex::new(std::time::Instant::now()));
    const POS_COOLDOWN: Duration = Duration::from_millis(100);

    let now = std::time::Instant::now();
    let should_update_pos = if throttle_position {
        let elapsed = last_pos.lock().unwrap().elapsed();
        elapsed >= POS_COOLDOWN
    } else {
        true
    };

    let cw = *state.card_width.lock().unwrap();
    let collapsed = state.collapsed_height.lock().unwrap().clone();
    let enabled = state.attach_enabled.lock().unwrap().clone();
    let whitelist = state.attach_whitelist.lock().unwrap().clone();
    let remember = state.attach_remember.lock().unwrap().clone();

    // Derive widget labels dynamically from registered attach state
    let labels: Vec<String> = enabled.keys().cloned().collect();

    // Get process name of target
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(target, Some(&mut pid)); }
    let process = get_process_name(pid);

    let seq_labels = state.sequence_labels.lock().unwrap().clone();

    let mut target_rect = RECT::default();
    if unsafe { GetWindowRect(target, &mut target_rect) }.is_err() { return; }

    let hmonitor = unsafe { MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST) };
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(hmonitor, &mut mi) }.as_bool() { return; }

    // cw is logical; target_rect is physical. Convert cw to physical pixels.
    let dpi = unsafe { GetDpiForSystem() }.max(96);
    let cw_phys = (cw as f32 * dpi as f32 / 96.0).round() as i32;

    let card_x = if target_rect.right + cw_phys <= mi.rcWork.right {
        target_rect.right
    } else if target_rect.left - cw_phys >= mi.rcWork.left {
        target_rect.left - cw_phys
    } else {
        mi.rcWork.right - cw_phys
    };
    let card_y = target_rect.top.max(mi.rcWork.top);

    let mut c = cache.lock().unwrap();
    let mut y_offset: i32 = 0;
    for label in &labels {
        if seq_labels.contains(label.as_str()) { continue; }
        if collapsed.contains_key(label.as_str()) { continue; }
        if enabled.get(label).copied() == Some(false) { continue; }
        let wl = whitelist.get(label).map(|v| v.as_slice()).unwrap_or(&[]);
        if !matches_whitelist(&process, wl) {
            let was_visible = c.get(label).map(|e| e.visible).unwrap_or(true);
            if was_visible {
                if let Some(win) = app_handle.get_webview_window(label) {
                    println!("[attach] hiding {} (whitelist mismatch, process={})", label, process);
                    let _ = win.hide();
                }
                c.insert(label.clone(), WidgetCache { visible: false, x: 0, y: 0 });
            }
            continue;
        }

        if let Some(win) = app_handle.get_webview_window(label) {
            let cached = c.get(label);
            let was_visible = cached.map(|e| e.visible).unwrap_or(false);
            if !was_visible {
                let _ = win.show();
                println!("[attach] showing {}", label);
            }
            // "remember" mode: skip repositioning, user has positioned this widget
            if remember.get(label).copied() == Some(true) {
                c.insert(label.clone(), WidgetCache { visible: true, x: 0, y: 0 });
                continue;
            }
            let new_x = card_x;
            let new_y = card_y + y_offset;
            let pos_changed = cached.map(|e| e.x != new_x || e.y != new_y).unwrap_or(true);
            if pos_changed && should_update_pos {
                let _ = win.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition { x: new_x, y: new_y },
                ));
                *last_pos.lock().unwrap() = now;
            }
            c.insert(label.clone(), WidgetCache { visible: true, x: new_x, y: new_y });
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

    // Thread 2: event-driven — only repositions when dirty flag is set
    // No polling needed since Thread 1's WinEventHook fires on window moves
    thread::spawn(move || {
        let mut last_fg: isize = 0;

        loop {
            if !*state.running.lock().unwrap() { break; }

            // Wait for dirty flag to be set (with 100ms timeout for foreground check)
            // This avoids busy-waiting while still catching foreground changes
            let mut waited = 0;
            while waited < 100 {
                if WINDOW_DIRTY.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
                waited += 10;
            }

            let fg = unsafe { GetForegroundWindow() };
            let fg_changed = fg.0 as isize != last_fg;
            let dirty = WINDOW_DIRTY.swap(false, std::sync::atomic::Ordering::Relaxed);

            if dirty || fg_changed {
                last_fg = fg.0 as isize;
                if !is_own_window(fg) {
                    reposition_widgets(&app_handle, fg, &state, dirty && !fg_changed);
                }
            }

            // Read browser URL periodically (independent of dirty/fg_changed)
            let process = get_process_name({
                let mut pid: u32 = 0;
                unsafe { GetWindowThreadProcessId(fg, Some(&mut pid)); }
                pid
            });
            if crate::page_notes::page_url::is_browser(&process) {
                static LAST_URL_READ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let last = LAST_URL_READ.load(std::sync::atomic::Ordering::Relaxed);
                if now.saturating_sub(last) >= 500 {
                    LAST_URL_READ.store(now, std::sync::atomic::Ordering::Relaxed);
                    if let Some(url) = crate::page_notes::page_url::read_browser_url(fg.0 as isize) {
                        *state.current_url.lock().unwrap() = url;
                    }
                }
            } else {
                let mut cu = state.current_url.lock().unwrap();
                if !cu.is_empty() { cu.clear(); }
            }
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start_attach_loop(_app_handle: tauri::AppHandle, _state: Arc<AttachState>) {}

#[allow(dead_code)]
pub fn stop_attach_loop(state: &AttachState) {
    *state.running.lock().unwrap() = false;
}
