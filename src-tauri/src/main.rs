// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod acrylic;
mod amkr;
mod browser_ext;
mod git;
mod git_watcher;
mod page_notes;
mod page_url;
mod plugins;
mod settings;
mod system_monitor;
mod tray;
mod window_attach;
mod clipboard_history;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use git_watcher::GitWatcherManager;
use window_attach::AttachState;
use system_monitor::SystemMonitorState;

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
    default_attach_remember: bool,
    default_whitelist: &[String],
) {
    let url = format!("index.html?widget={}", url_suffix);
    let plugin_id = url_suffix;

    // Try to restore saved state
    let saved = settings::load_settings(app.clone())
        .ok()
        .and_then(|s| s.window_states.get(plugin_id).cloned());

    let actual_height = saved.as_ref().and_then(|s| s.height).unwrap_or(height);
    println!("[create] {} saved_height={:?} manifest_height={} actual={}", plugin_id, saved.as_ref().and_then(|s| s.height), height, actual_height);

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
    // Restore attach_remember (saved value > plugin default)
    let initial_remember = saved.as_ref().and_then(|s| s.attach_remember).unwrap_or(default_attach_remember);
    {
        let mut arm = attach_state.attach_remember.lock().unwrap();
        arm.insert(label.to_string(), initial_remember);
    }

    // When attach is enabled, start hidden — the attach loop will show the window
    // when a matching foreground window is detected. This avoids a flash on startup.
    let start_visible = !initial_attach;

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
    .visible(start_visible);

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

/// Tauri command: set whether a widget uses "remember position" mode.
/// When true, the attach system only manages show/hide, not position.
#[tauri::command]
fn set_attach_remember(state: tauri::State<'_, Arc<AttachState>>, window_label: String, remember: bool) {
    let mut ar = state.attach_remember.lock().unwrap();
    ar.insert(window_label, remember);
}


/// Tauri command: get the current browser URL from the attach loop.
#[tauri::command]
fn get_browser_url(state: tauri::State<'_, Arc<AttachState>>) -> String {
    state.current_url.lock().unwrap().clone()
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
    default_attach_remember: bool,
    default_whitelist: Vec<String>,
) {
    let label = format!("widget-{}", plugin_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    create_widget_window(&app, &label, &title, &plugin_id, width, height, state.inner(), default_attach_enabled, default_attach_remember, &default_whitelist);
}

/// Tauri command: start watching a git repository for changes.
#[tauri::command]
fn watch_git_repo(watcher: tauri::State<'_, Arc<GitWatcherManager>>, repo_path: String) -> Result<(), String> {
    watcher.watch(&repo_path)
}

/// Tauri command: stop watching a git repository.
#[tauri::command]
fn unwatch_git_repo(watcher: tauri::State<'_, Arc<GitWatcherManager>>, repo_path: String) {
    watcher.unwatch(&repo_path);
}

/// Tauri command: open the global settings window.
#[tauri::command]
fn open_settings_window(app: tauri::AppHandle) {
    let label = "settings";
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let url = "index.html?widget=settings";
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("Galncelet 设置")
    .inner_size(400.0, 500.0)
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

#[derive(serde::Serialize)]
struct GitCommandResult {
    success: bool,
    stdout: String,
    stderr: String,
}

#[tauri::command]
fn exec_git_command(repo_root: String, command: String) -> Result<GitCommandResult, String> {
    let (success, stdout, stderr) = git::exec_git_command(&repo_root, &command)?;
    Ok(GitCommandResult { success, stdout, stderr })
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

            // Initialize git watcher manager
            let git_watcher = Arc::new(GitWatcherManager::new(handle.clone()));
            app.manage(git_watcher.clone());

            // AMKR WebSocket handle
            let amkr_ws_handle: amkr::AmkrWsHandle = Arc::new(Mutex::new(None));
            app.manage(amkr_ws_handle);

            // System monitor state
            let system_monitor = Arc::new(SystemMonitorState::new());
            app.manage(system_monitor);

            // Clipboard history
            let clipboard_state = Arc::new(clipboard_history::ClipboardHistoryState::new());
            app.manage(clipboard_state.clone());
            clipboard_history::start_monitor(clipboard_state);

            // Create widget windows from plugin manifests (zero hardcoded knowledge)
            let app_settings = settings::load_settings(handle.clone()).unwrap_or_default();
            println!("[setup] creating widget windows...");
            for manifest in plugins::load_manifests() {
                println!("[setup] loading plugin: {} (visible={})", manifest.id, app_settings.panel_visibility.get(&manifest.id).copied().unwrap_or(true));
                if !app_settings.panel_visibility.get(&manifest.id).copied().unwrap_or(true) {
                    continue;
                }
                let label = format!("widget-{}", manifest.id);
                let w = manifest.default_width.unwrap_or(360.0);
                let h = manifest.default_height.unwrap_or(600.0);
                let attach = manifest.default_attach_enabled.unwrap_or(true);
                let remember = manifest.default_attach_remember.unwrap_or(false);
                let wl = manifest.default_whitelist.clone().unwrap_or_default();
                create_widget_window(&handle, &label, &manifest.title, &manifest.id, w, h, &attach_state, attach, remember, &wl);
            }

            // List all created windows
            for (label, _win) in handle.webview_windows() {
                println!("[setup] window created: {}", label);
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

            // Create settings window (hidden by default)
            let settings_url = "index.html?widget=settings";
            let settings_win = tauri::WebviewWindowBuilder::new(
                &handle,
                "settings",
                tauri::WebviewUrl::App(settings_url.into()),
            )
            .title("Galncelet 设置")
            .inner_size(400.0, 500.0)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .resizable(false)
            .maximizable(false)
            .skip_taskbar(true)
            .visible(false)
            .build()
            .expect("failed to create settings window");
            let settings_handle = settings_win.clone();
            settings_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = settings_handle.hide();
                }
            });

            // Browser extension setup (copy to app data, write registry)
            browser_ext::setup(&handle);

            // System tray
            tray::setup(app).expect("failed to setup system tray");

            // Window attachment loop
            let app_handle = app.handle().clone();
            window_attach::start_attach_loop(app_handle, attach_state);

            // Start page-notes WebSocket server
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = page_notes::start_ws_server(app_handle).await {
                    eprintln!("[page-notes] Failed to start WebSocket server: {}", e);
                }
            });

            // Start AMKR event WebSocket client
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let ws_handle = app_handle.state::<amkr::AmkrWsHandle>();
                if let Err(e) = amkr::start_amkr_ws(app_handle.clone(), ws_handle).await {
                    eprintln!("[amkr] Failed to start WebSocket client: {}", e);
                }
            });

            // Auto-watch saved repos
            let app_settings = settings::load_settings(handle.clone()).unwrap_or_default();
            for repo in &app_settings.saved_repos {
                let _ = git_watcher.watch(repo);
            }

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
            settings::set_plugin_visible,
            update_card_width,
            set_body_collapsed,
            set_attach_enabled,
            set_attach_whitelist,
            set_attach_remember,
            get_browser_url,
            clipboard_history::get_clipboard_history,
            clipboard_history::copy_to_clipboard,
            clipboard_history::delete_clipboard_entry,
            clipboard_history::clear_clipboard_history,
            window_attach::list_visible_windows,
            window_attach::snap_widget,
            window_attach::unsnap_widget,
            window_attach::get_snap_info,
            window_attach::get_all_widget_rects,
            window_attach::move_snap_group,
            create_plugin_window,
            open_manage_window,
            open_settings_window,
            open_plugin_settings,
            watch_git_repo,
            unwatch_git_repo,
            exec_git_command,
            amkr::fetch_amkr_metrics,
            amkr::generate_commit_message,
            amkr::get_amkr_models,
            amkr::set_amkr_unified_model,
            amkr::start_amkr_ws,
            amkr::stop_amkr_ws,
            page_notes::load_page_notes,
            page_notes::save_page_notes,
            page_notes::get_ws_port,
            browser_ext::open_extension_dir,
            browser_ext::launch_browser_with_extension,
            system_monitor::fetch_system_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
