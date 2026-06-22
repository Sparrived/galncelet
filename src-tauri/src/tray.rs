use crate::plugins;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// 打开或显示插件设置窗口
fn open_settings(app: &tauri::AppHandle, plugin_id: &str) {
    let label = format!("settings-{}", plugin_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let url = format!("index.html?widget=plugin_settings&plugin={}", plugin_id);
    let win = tauri::WebviewWindowBuilder::new(
        app,
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

    let handle = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = handle.hide();
        }
    });
}

/// Set up the system tray icon with context menu.
/// Plugin entries are dynamically generated from manifests.
pub fn setup(app: &tauri::App, manifests: &[plugins::PluginManifest]) -> Result<(), Box<dyn std::error::Error>> {
    // Dynamically build plugin settings menu items from manifests
    let plugin_items: Vec<MenuItem<tauri::Wry>> = manifests
        .iter()
        .map(|m| {
            let id = format!("plugin_settings:{}", m.id);
            let label = format!("{} 设置", m.title);
            MenuItem::with_id(app, id, label, true, None::<&str>)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let global_settings = MenuItem::with_id(app, "global_settings", "全局设置", true, None::<&str>)?;
    let manage = MenuItem::with_id(app, "manage", "插件管理", true, None::<&str>)?;
    let hide_all = MenuItem::with_id(app, "hide_all", "隐藏全部", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let mut menu_items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = Vec::new();
    for item in &plugin_items {
        menu_items.push(item);
    }
    menu_items.extend([&global_settings as &dyn tauri::menu::IsMenuItem<tauri::Wry>, &manage, &hide_all, &quit]);

    let menu = Menu::with_items(app, &menu_items)?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Galncelet")
        .menu(&menu)
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref().to_string();
            if let Some(plugin_id) = id.strip_prefix("plugin_settings:") {
                open_settings(&app, plugin_id);
            } else {
                match id.as_str() {
                    "global_settings" => {
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "manage" => {
                        if let Some(win) = app.get_webview_window("manage") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "hide_all" => {
                        for (_, win) in app.webview_windows() {
                            let _ = win.hide();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("manage") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
