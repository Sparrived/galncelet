// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Non-plugin framework modules
mod acrylic;
mod plugins;
mod runtime_addons;
mod settings;
mod tray;
mod updater;
mod window_attach;

// Auto-generated plugin modules
mod _plugins;
use _plugins::{amkr, browser_ext, clipboard_history, git, music_player, page_notes, system_monitor};

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
    default_attach_remember: bool,
    default_whitelist: &[String],
    initial_visible: Option<bool>,
) {
    let url = format!("index.html?widget={}", url_suffix);
    let plugin_id = url_suffix;

    let saved = settings::load_settings(app.clone())
        .ok()
        .and_then(|s| s.window_states.get(plugin_id).cloned());

    let actual_height = saved.as_ref().and_then(|s| s.height).unwrap_or(height);
    println!("[create] {} saved_height={:?} manifest_height={} actual={}", plugin_id, saved.as_ref().and_then(|s| s.height), height, actual_height);

    let initial_attach = saved.as_ref().and_then(|s| s.attach_enabled).unwrap_or(default_attach_enabled);
    {
        let mut ae = attach_state.attach_enabled.lock().unwrap();
        ae.insert(label.to_string(), initial_attach);
    }
    let saved_wl = saved.as_ref().and_then(|s| s.whitelist.clone()).filter(|v| !v.is_empty());
    let whitelist = saved_wl.unwrap_or(default_whitelist.to_vec());
    {
        let mut wlm = attach_state.attach_whitelist.lock().unwrap();
        wlm.insert(label.to_string(), whitelist);
    }
    let initial_remember = saved.as_ref().and_then(|s| s.attach_remember).unwrap_or(default_attach_remember);
    {
        let mut arm = attach_state.attach_remember.lock().unwrap();
        arm.insert(label.to_string(), initial_remember);
    }

    let start_visible = initial_visible.unwrap_or(!initial_attach);

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

    if let Some(ref s) = saved {
        if let (Some(x), Some(y)) = (s.x, s.y) {
            builder = builder.position(x, y);
        }
    }

    let win = builder.build().expect(&format!("failed to create window {}", label));

    let win_handle = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_handle.hide();
        }
    });
}

// ─── Framework Tauri commands ──────────────────────────────────────

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
            let _ = win.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: phys_w,
                height: h,
            }));
        } else if let Some(eh) = expand_height {
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

#[tauri::command]
fn set_attach_whitelist(state: tauri::State<'_, Arc<AttachState>>, window_label: String, patterns: Vec<String>) {
    let mut wl = state.attach_whitelist.lock().unwrap();
    wl.insert(window_label, patterns);
}

#[tauri::command]
fn set_attach_remember(state: tauri::State<'_, Arc<AttachState>>, window_label: String, remember: bool) {
    let mut ar = state.attach_remember.lock().unwrap();
    ar.insert(window_label, remember);
}

#[tauri::command]
fn get_browser_url(state: tauri::State<'_, Arc<AttachState>>) -> String {
    state.current_url.lock().unwrap().clone()
}

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
    create_widget_window(&app, &label, &title, &plugin_id, width, height, state.inner(), default_attach_enabled, default_attach_remember, &default_whitelist, None);
}

#[tauri::command]
fn create_runtime_addon_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AttachState>>,
    addon_id: String,
) -> Result<(), String> {
    let addon = runtime_addons::load_runtime_addon(&app, &addon_id)?;
    let label = format!("runtime-addon-{}", addon.id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    let entry = runtime_addons::addon_entry_path(&app, &addon.id)?;
    let entry_url = tauri::Url::from_file_path(&entry)
        .map_err(|_| format!("Failed to convert addon entry to file URL: {}", entry.display()))?;
    let saved = settings::load_settings(app.clone())
        .ok()
        .and_then(|s| s.window_states.get(&addon.id).cloned());
    let width = addon.default_width.unwrap_or(360.0);
    let height = saved.as_ref().and_then(|s| s.height).unwrap_or(addon.default_height.unwrap_or(600.0));
    let attach = saved.as_ref().and_then(|s| s.attach_enabled).unwrap_or(addon.default_attach_enabled.unwrap_or(false));
    let remember = saved.as_ref().and_then(|s| s.attach_remember).unwrap_or(addon.default_attach_remember.unwrap_or(false));
    let whitelist = saved
        .as_ref()
        .and_then(|s| s.whitelist.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| addon.default_whitelist.clone());

    {
        state.attach_enabled.lock().unwrap().insert(label.clone(), attach);
        state.attach_remember.lock().unwrap().insert(label.clone(), remember);
        state.attach_whitelist.lock().unwrap().insert(label.clone(), whitelist);
    }

    let mut builder = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::External(entry_url),
    )
    .title(&addon.title)
    .inner_size(width, height)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .maximizable(false)
    .skip_taskbar(true)
    .visible(!attach);

    if let Some(ref s) = saved {
        if let (Some(x), Some(y)) = (s.x, s.y) {
            builder = builder.position(x, y);
        }
    }

    let win = builder.build().map_err(|e| format!("failed to create runtime addon window: {e}"))?;
    let win_handle = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_handle.hide();
        }
    });
    Ok(())
}

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

// ─── Plugin command wrappers (Tauri requires functions in scope) ───

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
    let folder = app.dialog().file().set_title("选择文件夹").blocking_pick_folder();
    Ok(folder.map(|p| p.to_string()))
}

#[tauri::command]
fn stage_file(repo_root: String, file_path: String) -> Result<(), String> { git::stage_file(&repo_root, &file_path) }

#[tauri::command]
fn stage_all(repo_root: String) -> Result<(), String> { git::stage_all(&repo_root) }

#[tauri::command]
fn unstage_file(repo_root: String, file_path: String) -> Result<(), String> { git::unstage_file(&repo_root, &file_path) }

#[tauri::command]
fn discard_file(repo_root: String, file_path: String, status_code: String) -> Result<(), String> { git::discard_file(&repo_root, &file_path, &status_code) }

#[tauri::command]
fn untrack_file(repo_root: String, file_path: String) -> Result<(), String> { git::untrack_file(&repo_root, &file_path) }

#[tauri::command]
fn commit(repo_root: String, message: String) -> Result<String, String> { git::commit(&repo_root, &message) }

#[tauri::command]
fn pull(repo_root: String) -> Result<String, String> { git::pull(&repo_root) }

#[tauri::command]
fn push(repo_root: String) -> Result<String, String> { git::push(&repo_root) }

#[tauri::command]
fn git_fetch(repo_root: String) -> Result<String, String> { git::git_fetch(&repo_root) }

#[tauri::command]
fn list_branches(repo_root: String) -> Result<Vec<git::GitBranch>, String> { git::list_branches(&repo_root) }

#[tauri::command]
fn checkout_branch(repo_root: String, branch: String) -> Result<String, String> { git::checkout_branch(&repo_root, &branch) }

#[tauri::command]
fn git_log(repo_root: String, max_count: Option<usize>) -> Result<Vec<git::GitLogEntry>, String> { git::git_log(&repo_root, max_count.unwrap_or(50)) }

#[tauri::command]
fn list_submodules(repo_root: String) -> Vec<git::SubmoduleInfo> { git::list_submodules(&repo_root) }

#[tauri::command]
fn list_remotes(repo_root: String) -> Result<Vec<git::RemoteInfo>, String> { git::list_remotes(&repo_root) }

#[tauri::command]
fn add_remote(repo_root: String, name: String, url: String) -> Result<(), String> { git::add_remote(&repo_root, &name, &url) }

#[tauri::command]
fn remove_remote(repo_root: String, name: String) -> Result<(), String> { git::remove_remote(&repo_root, &name) }

#[tauri::command]
fn watch_git_repo(watcher: tauri::State<'_, Arc<git::git_watcher::GitWatcherManager>>, repo_path: String) -> Result<(), String> {
    watcher.watch(&repo_path)
}

#[tauri::command]
fn unwatch_git_repo(watcher: tauri::State<'_, Arc<git::git_watcher::GitWatcherManager>>, repo_path: String) {
    watcher.unwatch(&repo_path);
}

// ─── Main ──────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            // Framework state
            let attach_state = Arc::new(AttachState::new());
            app.manage(attach_state.clone());

            // Initialize plugins (auto-generated)
            _plugins::setup_all(&handle);

            // Create widget windows from plugin manifests
            let manifests = plugins::load_manifests();
            let mut app_settings = settings::load_settings(handle.clone()).unwrap_or_default();
            app_settings.ensure_plugin_visibility(&manifests);
            println!("[setup] creating widget windows...");
            // Register sequence widget labels so attach loop skips them
            {
                let mut sl = attach_state.sequence_labels.lock().unwrap();
                for pid in &app_settings.widget_sequence {
                    sl.insert(format!("widget-{}", pid));
                }
            }
            for manifest in &manifests {
                let visible = app_settings.panel_visibility.get(&manifest.id).copied().unwrap_or(false);
                let sequence_index = app_settings.widget_sequence.iter().position(|id| id == &manifest.id);
                let in_sequence = sequence_index.is_some();
                println!("[setup] loading plugin: {} (visible={}, seq={})", manifest.id, visible, in_sequence);
                // Create window if visible OR if it's in the widget sequence
                if !visible && !in_sequence {
                    continue;
                }
                let label = format!("widget-{}", manifest.id);
                let w = manifest.default_width.unwrap_or(360.0);
                let h = manifest.default_height.unwrap_or(600.0);
                let attach = manifest.default_attach_enabled.unwrap_or(true);
                let remember = manifest.default_attach_remember.unwrap_or(false);
                let wl = manifest.default_whitelist.clone().unwrap_or_default();
                let initial_visible = sequence_index.map(|idx| idx == 0);
                create_widget_window(&handle, &label, &manifest.title, &manifest.id, w, h, &attach_state, attach, remember, &wl, initial_visible);
            }

            // Create runtime addon windows from user-provided manifests.
            match runtime_addons::load_runtime_addons(&handle) {
                Ok(runtime_manifests) => {
                    for manifest in &runtime_manifests {
                        app_settings.panel_visibility.entry(manifest.id.clone()).or_insert(false);
                    }
                    let _ = settings::save_settings(handle.clone(), app_settings.clone());
                    for manifest in runtime_manifests {
                        let visible = app_settings.panel_visibility.get(&manifest.id).copied().unwrap_or(false);
                        if !visible {
                            continue;
                        }
                        let _ = create_runtime_addon_window(
                            handle.clone(),
                            app.state::<Arc<AttachState>>(),
                            manifest.id.clone(),
                        );
                    }
                }
                Err(e) => eprintln!("[runtime-addons] failed to load: {e}"),
            }

            // List all created windows
            for (label, _win) in handle.webview_windows() {
                println!("[setup] window created: {}", label);
            }

            // Management window (hidden)
            let manage_win = tauri::WebviewWindowBuilder::new(
                &handle, "manage",
                tauri::WebviewUrl::App("index.html?widget=manage".into()),
            )
            .title("Galncelet 管理").inner_size(400.0, 500.0)
            .transparent(true).decorations(false).always_on_top(true)
            .resizable(false).maximizable(false).skip_taskbar(true).visible(false)
            .build().expect("failed to create manage window");
            let h = manage_win.clone();
            manage_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = h.hide(); }
            });

            // Settings window (hidden)
            let settings_win = tauri::WebviewWindowBuilder::new(
                &handle, "settings",
                tauri::WebviewUrl::App("index.html?widget=settings".into()),
            )
            .title("Galncelet 设置").inner_size(400.0, 500.0)
            .transparent(true).decorations(false).always_on_top(true)
            .resizable(false).maximizable(false).skip_taskbar(true).visible(false)
            .build().expect("failed to create settings window");
            let h = settings_win.clone();
            settings_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = h.hide(); }
            });

            // System tray (dynamically built from plugin manifests)
            tray::setup(app, &manifests).expect("failed to setup system tray");

            // Runtime addon folder watcher
            runtime_addons::start_runtime_addon_watcher(handle.clone());

            // Window attachment loop
            let app_handle = app.handle().clone();
            window_attach::start_attach_loop(app_handle, attach_state);

            // Register plugin hotkeys
            let app_handle = app.handle().clone();
            settings::register_all_hotkeys(&app_handle, &app_settings);

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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Framework commands
            update_card_width,
            set_body_collapsed,
            set_attach_enabled,
            set_attach_whitelist,
            set_attach_remember,
            get_browser_url,
            create_plugin_window,
            create_runtime_addon_window,
            runtime_addons::list_runtime_addons,
            runtime_addons::get_runtime_addons_dir,
            runtime_addons::open_runtime_addons_dir,
            runtime_addons::invoke_runtime_addon,
            runtime_addons::runtime_addon_storage_get,
            runtime_addons::runtime_addon_storage_set,
            runtime_addons::runtime_addon_storage_delete,
            open_manage_window,
            open_settings_window,
            open_plugin_settings,
            select_folder,
            // Settings
            settings::load_settings,
            settings::save_settings,
            settings::set_start_on_boot,
            settings::save_window_state,
            settings::set_plugin_visible,
            // Updates
            updater::check_for_updates,
            // Window attach
            window_attach::list_visible_windows,
            // Git plugin wrappers (defined in main.rs)
            get_status,
            get_file_diff,
            exec_git_command,
            stage_file,
            stage_all,
            unstage_file,
            discard_file,
            untrack_file,
            commit,
            pull,
            push,
            git_fetch,
            list_branches,
            checkout_branch,
            git_log,
            list_submodules,
            list_remotes,
            add_remote,
            remove_remote,
            settings::set_plugin_hotkey,
            settings::set_widget_sequence,
            settings::set_sequence_hotkey,
            watch_git_repo,
            unwatch_git_repo,
            // AMKR plugin
            amkr::fetch_amkr_metrics,
            amkr::generate_commit_message,
            amkr::get_amkr_models,
            amkr::set_amkr_unified_model,
            amkr::start_amkr_ws,
            amkr::stop_amkr_ws,
            // Page Notes plugin
            page_notes::load_page_notes,
            page_notes::save_page_notes,
            page_notes::get_ws_port,
            // Browser Extension
            browser_ext::open_extension_dir,
            browser_ext::launch_browser_with_extension,
            // System Monitor plugin
            system_monitor::fetch_system_metrics,
            // Clipboard History plugin
            clipboard_history::get_clipboard_history,
            clipboard_history::copy_to_clipboard,
            clipboard_history::delete_clipboard_entry,
            clipboard_history::clear_clipboard_history,
            // Music Player plugin
            music_player::get_media_info,
            music_player::media_control,
            music_player::get_media_sessions,
            music_player::select_media_session,
            music_player::get_lyrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
