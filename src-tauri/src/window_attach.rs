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

/// Which edge of the target widget to snap to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SnapEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// A widget's snap relationship: snapped to another widget's edge.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapTarget {
    pub target_label: String,
    pub edge: SnapEdge,
    /// The perpendicular coordinate at snap time (x for top/bottom, y for left/right).
    pub offset: i32,
}

/// Physical rect of a widget window.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WidgetRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

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
    /// Widget-to-widget snap relationships: label → snap target.
    pub snap_groups: Arc<Mutex<HashMap<String, SnapTarget>>>,
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
            snap_groups: Arc::new(Mutex::new(HashMap::new())),
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

/// Reposition all enabled, non-collapsed widgets next to the given window.
/// Widgets with attach_enabled = false are not affected by this logic.
#[cfg(target_os = "windows")]
fn reposition_widgets(app_handle: &tauri::AppHandle, target: HWND, state: &AttachState) {
    let cw = *state.card_width.lock().unwrap();
    let collapsed = state.collapsed_height.lock().unwrap().clone();
    let enabled = state.attach_enabled.lock().unwrap().clone();
    let whitelist = state.attach_whitelist.lock().unwrap().clone();
    let remember = state.attach_remember.lock().unwrap().clone();

    // Derive widget labels dynamically from registered attach state
    let labels: Vec<String> = enabled.keys().cloned().collect();
    // Debug: check which windows actually exist
    static WIN_CHECK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
    if WIN_CHECK.swap(false, std::sync::atomic::Ordering::Relaxed) {
        for label in &labels {
            let exists = app_handle.get_webview_window(label).is_some();
            println!("[attach] window {} exists={}", label, exists);
        }
    }

    // Get process name of target
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(target, Some(&mut pid)); }
    let process = get_process_name(pid);

    // Check which attach-enabled widgets should be visible
    let any_visible = labels.iter().any(|label| {
        if collapsed.contains_key(label.as_str()) { return false; }
        if enabled.get(label).copied() == Some(false) { return false; }
        let wl = whitelist.get(label).map(|v| v.as_slice()).unwrap_or(&[]);
        matches_whitelist(&process, wl)
    });

    if !any_visible {
        for label in &labels {
            if collapsed.contains_key(label.as_str()) { continue; }
            if enabled.get(label).copied() == Some(false) { continue; }
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

    let mut y_offset: i32 = 0;
    for label in &labels {
        if collapsed.contains_key(label.as_str()) { continue; }
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
            println!("[attach] showing {}", label);
            // "remember" mode: skip repositioning, user has positioned this widget
            if remember.get(label).copied() == Some(true) {
                continue;
            }
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
                    reposition_widgets(&app_handle, fg, &state);
                }
            }

            // Read browser URL periodically (independent of dirty/fg_changed)
            let process = get_process_name({
                let mut pid: u32 = 0;
                unsafe { GetWindowThreadProcessId(fg, Some(&mut pid)); }
                pid
            });
            if crate::page_url::is_browser(&process) {
                static LAST_URL_READ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let last = LAST_URL_READ.load(std::sync::atomic::Ordering::Relaxed);
                if now.saturating_sub(last) >= 500 {
                    LAST_URL_READ.store(now, std::sync::atomic::Ordering::Relaxed);
                    if let Some(url) = crate::page_url::read_browser_url(fg.0 as isize) {
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

// ─── Widget-to-Widget Snap Commands ───

fn get_widget_rect(app: &tauri::AppHandle, label: &str) -> Option<WidgetRect> {
    let win = app.get_webview_window(label)?;
    let pos = win.outer_position().ok()?;
    let size = win.outer_size().ok()?;
    Some(WidgetRect { x: pos.x, y: pos.y, w: size.width as i32, h: size.height as i32 })
}

/// Snap a widget to another widget's edge. Moves the widget to align.
#[tauri::command]
pub fn snap_widget(
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<AttachState>>,
    label: String,
    target_label: String,
    edge: SnapEdge,
    offset: i32,
) {
    // Record snap relationship
    state.snap_groups.lock().unwrap().insert(label.clone(), SnapTarget {
        target_label: target_label.clone(),
        edge,
        offset,
    });

    // Reposition widget to align with target edge
    if let (Some(_my), Some(target)) = (get_widget_rect(&app, &label), get_widget_rect(&app, &target_label)) {
        let (x, y) = match edge {
            SnapEdge::Bottom => (target.x, target.y + target.h),
            SnapEdge::Top => (target.x, target.y - _my.h),
            SnapEdge::Right => (target.x + target.w, target.y),
            SnapEdge::Left => (target.x - _my.w, target.y),
        };
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition { x, y },
            ));
        }
    }
}

/// Remove snap relationship for a widget.
#[tauri::command]
pub fn unsnap_widget(
    state: tauri::State<'_, std::sync::Arc<AttachState>>,
    label: String,
) {
    state.snap_groups.lock().unwrap().remove(&label);
}

/// Get snap info for a widget.
#[tauri::command]
pub fn get_snap_info(
    state: tauri::State<'_, std::sync::Arc<AttachState>>,
    label: String,
) -> Option<SnapTarget> {
    state.snap_groups.lock().unwrap().get(&label).cloned()
}

/// Get physical rects of all visible widget windows.
#[tauri::command]
pub fn get_all_widget_rects(app: tauri::AppHandle) -> HashMap<String, WidgetRect> {
    let mut rects = HashMap::new();
    for (label, win) in app.webview_windows() {
        if label.starts_with("widget-") {
            if win.is_visible().unwrap_or(false) {
                if let Ok(pos) = win.outer_position() {
                    if let Ok(size) = win.outer_size() {
                        rects.insert(label, WidgetRect {
                            x: pos.x, y: pos.y,
                            w: size.width as i32, h: size.height as i32,
                        });
                    }
                }
            }
        }
    }
    rects
}

/// Move a widget and all widgets snapped to it by (dx, dy) physical pixels.
#[tauri::command]
pub fn move_snap_group(
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<AttachState>>,
    label: String,
    dx: i32,
    dy: i32,
) {
    let snap = state.snap_groups.lock().unwrap();
    // Find all widgets snapped TO this widget (reverse lookup)
    let dependents: Vec<String> = snap.iter()
        .filter(|(_, t)| t.target_label == label)
        .map(|(k, _)| k.clone())
        .collect();
    drop(snap);

    for dep in &dependents {
        if let Some(win) = app.get_webview_window(dep) {
            if let Ok(pos) = win.outer_position() {
                let _ = win.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition { x: pos.x + dx, y: pos.y + dy },
                ));
                // Recurse for chains (A→B→C)
                move_snap_group_inner(&app, &state, dep.clone(), dx, dy);
            }
        }
    }
}

fn move_snap_group_inner(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, std::sync::Arc<AttachState>>,
    label: String,
    dx: i32,
    dy: i32,
) {
    let snap = state.snap_groups.lock().unwrap();
    let dependents: Vec<String> = snap.iter()
        .filter(|(_, t)| t.target_label == label)
        .map(|(k, _)| k.clone())
        .collect();
    drop(snap);

    for dep in &dependents {
        if let Some(win) = app.get_webview_window(dep) {
            if let Ok(pos) = win.outer_position() {
                let _ = win.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition { x: pos.x + dx, y: pos.y + dy },
                ));
            }
        }
    }
}
