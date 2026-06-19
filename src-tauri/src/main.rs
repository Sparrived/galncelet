// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod acrylic;
mod amkr;
mod git;
mod plugins;
mod settings;
mod tray;
mod window_attach;

use std::sync::Arc;
use tauri::Manager;
use window_attach::AttachState;

/// Create a widget window with standard glass-panel styling.
/// If saved position/height exist in settings, they are restored.
fn create_widget_window(
    app: &tauri::AppHandle,
    label: &str,
    title: &str,
    url_suffix: &str,
    width: f64,
    height: f64,
    attach_state: &AttachState,
    default_attach_enabled: bool,
    default_whitelist: &[String],
) {
    let url = format!("index.html?widget={}", url_suffix);
    let plugin_id = url_suffix;

    // Try to restore saved state
    let saved = settings::load_settings(app.clone())
        .ok()
        .and_then(|s| s.window_states.get(plugin_id).cloned());

    let actual_height = saved.as_ref().and_then(|s| s.height).unwrap_or(height);

    // Determine initial attach enabled: saved value > plugin default
    let initial_attach = saved.as_ref().and_then(|s| s.attach_enabled).unwrap_or(default_attach_enabled);

    // Initialize attach state immediately so the loop picks it up from the start
    {
        let mut ae = attach_state.attach_enabled.lock().unwrap();
        ae.insert(label.to_string(), initial_attach);
    }
    // Restore whitelist: use saved if non-empty, otherwise plugin default
    let saved_wl = saved.as_ref().and_then(|s| s.whitelist.clone()).filter(|v| !v.is_empty());
    let whitelist = saved_wl.unwrap_or(default_whitelist.to_vec());
    {
        let mut wlm = attach_state.attach_whitelist.lock().unwrap();
        wlm.insert(label.to_string(), whitelist);
    }

    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title(title)
    .inner_size(width, actual_height)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .maximizable(false)
    .skip_taskbar(true)
    .visible(true);

    // Restore saved position
    if let Some(ref s) = saved {
        if let (Some(x), Some(y)) = (s.x, s.y) {
            builder = builder.position(x, y);
        }
    }

    let win = builder.build().expect(&format!("failed to create window {}", label));

    // Intercept close → hide to tray instead of destroying
    let win_handle = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_handle.hide();
        }
    });
}

// ─── Tauri commands ───────────────────────────────────────────────

#[tauri::command]
fn update_card_width(state: tauri::State<'_, Arc<AttachState>>, width: u32) {
    let mut cw = state.card_width.lock().unwrap();
    *cw = width as i32;
}

#[tauri::command]
fn set_body_collapsed(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AttachState>>,
    window_label: String,
    height: Option<u32>,
    expand_height: Option<u32>,
) {
    let mut ch = state.collapsed_height.lock().unwrap();
    if let Some(h) = height {
        ch.insert(window_label.clone(), h as i32);
    } else {
        ch.remove(&window_label);
    }
    drop(ch);

    if let Some(win) = app.get_webview_window(&window_label) {
        let scale = win.scale_factor().unwrap_or(1.0);
        let cw = *state.card_width.lock().unwrap();
        let phys_w = (cw as f64 * scale) as u32;
        if let Some(h) = height {
            // Collapse: resize to header height
            let _ = win.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: phys_w,
                height: h,
            }));
        } else if let Some(eh) = expand_height {
            // Expand: restore full size
            let _ = win.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: phys_w,
                height: eh,
            }));
        }
    }
}

#[tauri::command]
fn set_attach_enabled(state: tauri::State<'_, Arc<AttachState>>, window_label: String, enabled: bool) {
    let mut ae = state.attach_enabled.lock().unwrap();
    ae.insert(window_label, enabled);
}

/// Tauri command: set the attach whitelist for a specific window.
/// Empty list = no restriction (attach to any window).
#[tauri::command]
fn set_attach_whitelist(state: tauri::State<'_, Arc<AttachState>>, window_label: String, patterns: Vec<String>) {
    let mut wl = state.attach_whitelist.lock().unwrap();
    wl.insert(window_label, patterns);
}

/// Tauri command: create a plugin widget window on demand.
/// If the window already exists, just show and focus it.
#[tauri::command]
fn create_plugin_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AttachState>>,
    plugin_id: String,
    title: String,
    width: f64,
    height: f64,
    default_attach_enabled: bool,
    default_whitelist: Vec<String>,
) {
    let label = format!("widget-{}", plugin_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    create_widget_window(&app, &label, &title, &plugin_id, width, height, state.inner(), default_attach_enabled, &default_whitelist);
}

/// Tauri command: open a plugin settings window.
#[tauri::command]
fn open_plugin_settings(app: tauri::AppHandle, plugin_id: String) {
    let label = format!("settings-{}", plugin_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let url = format!("index.html?widget=plugin_settings&plugin={}", plugin_id);
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title(format!("{} 设置", plugin_id))
    .inner_size(380.0, 500.0)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .maximizable(false)
    .skip_taskbar(true)
    .visible(true)
    .build()
    .expect("failed to create settings window");

    let win_handle = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_handle.hide();
        }
    });
}

/// Tauri command: open the management window.
#[tauri::command]
fn open_manage_window(app: tauri::AppHandle) {
    let label = "manage";
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let url = "index.html?widget=manage";
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("Galncelet 管理")
    .inner_size(400.0, 500.0)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .maximizable(false)
    .skip_taskbar(true)
    .visible(true)
    .build()
    .expect("failed to create manage window");

    let win_handle = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_handle.hide();
        }
    });
}

#[tauri::command]
fn get_status(repo_path: Option<String>) -> Result<git::GitStatus, String> {
    git::get_status(repo_path.as_deref())
}

#[tauri::command]
fn get_file_diff(repo_root: String, file_path: String, staged: bool) -> Result<git::GitDiff, String> {
    git::get_file_diff(&repo_root, &file_path, staged)
}

#[tauri::command]
async fn select_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let folder = app.dialog().file().set_title("选择 Git 仓库目录").blocking_pick_folder();
    Ok(folder.map(|p| p.to_string()))
}

#[tauri::command]
fn stage_file(repo_root: String, file_path: String) -> Result<(), String> {
    git::stage_file(&repo_root, &file_path)
}

#[tauri::command]
fn stage_all(repo_root: String) -> Result<(), String> {
    git::stage_all(&repo_root)
}

#[tauri::command]
fn unstage_file(repo_root: String, file_path: String) -> Result<(), String> {
    git::unstage_file(&repo_root, &file_path)
}

#[tauri::command]
fn discard_file(repo_root: String, file_path: String, status_code: String) -> Result<(), String> {
    git::discard_file(&repo_root, &file_path, &status_code)
}

#[tauri::command]
fn untrack_file(repo_root: String, file_path: String) -> Result<(), String> {
    git::untrack_file(&repo_root, &file_path)
}

#[tauri::command]
fn commit(repo_root: String, message: String) -> Result<String, String> {
    git::commit(&repo_root, &message)
}

#[tauri::command]
fn pull(repo_root: String) -> Result<String, String> {
    git::pull(&repo_root)
}

#[tauri::command]
fn push(repo_root: String) -> Result<String, String> {
    git::push(&repo_root)
}

#[tauri::command]
fn git_fetch(repo_root: String) -> Result<String, String> {
    git::git_fetch(&repo_root)
}

#[tauri::command]
fn list_branches(repo_root: String) -> Result<Vec<git::GitBranch>, String> {
    git::list_branches(&repo_root)
}

#[tauri::command]
fn checkout_branch(repo_root: String, branch: String) -> Result<String, String> {
    git::checkout_branch(&repo_root, &branch)
}

#[tauri::command]
fn git_log(repo_root: String, max_count: Option<usize>) -> Result<Vec<git::GitLogEntry>, String> {
    git::git_log(&repo_root, max_count.unwrap_or(50))
}

#[tauri::command]
fn list_submodules(repo_root: String) -> Vec<git::SubmoduleInfo> {
    git::list_submodules(&repo_root)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // Initialize attach state first (before creating windows)
            let attach_state = Arc::new(AttachState::new());
            app.manage(attach_state.clone());

            // Create widget windows from plugin manifests (zero hardcoded knowledge)
            let app_settings = settings::load_settings(handle.clone()).unwrap_or_default();
            for manifest in plugins::load_manifests() {
                if !app_settings.panel_visibility.get(&manifest.id).copied().unwrap_or(true) {
                    continue;
                }
                let label = format!("widget-{}", manifest.id);
                let w = manifest.default_width.unwrap_or(360.0);
                let h = manifest.default_height.unwrap_or(600.0);
                let attach = manifest.default_attach_enabled.unwrap_or(true);
                let wl = manifest.default_whitelist.clone().unwrap_or_default();
                create_widget_window(&handle, &label, &manifest.title, &manifest.id, w, h, &attach_state, attach, &wl);
            }

            // Create management window (hidden by default)
            let manage_url = "index.html?widget=manage";
            let manage_win = tauri::WebviewWindowBuilder::new(
                &handle,
                "manage",
                tauri::WebviewUrl::App(manage_url.into()),
            )
            .title("Galncelet 管理")
            .inner_size(400.0, 500.0)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .resizable(false)
            .maximizable(false)
            .skip_taskbar(true)
            .visible(false)
            .build()
            .expect("failed to create manage window");
            let manage_handle = manage_win.clone();
            manage_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = manage_handle.hide();
                }
            });

            // System tray
            tray::setup(app).expect("failed to setup system tray");

            // Window attachment loop
            let app_handle = app.handle().clone();
            window_attach::start_attach_loop(app_handle, attach_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_file_diff,
            select_folder,
            stage_file,
            stage_all,
            unstage_file,
            discard_file,
            commit,
            pull,
            push,
            git_fetch,
            list_branches,
            checkout_branch,
            git_log,
            list_submodules,
            untrack_file,
            settings::load_settings,
            settings::save_settings,
            settings::save_window_state,
            update_card_width,
            set_body_collapsed,
            set_attach_enabled,
            set_attach_whitelist,
            window_attach::list_visible_windows,
            create_plugin_window,
            open_manage_window,
            open_plugin_settings,
            amkr::fetch_amkr_metrics,
            amkr::generate_commit_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
