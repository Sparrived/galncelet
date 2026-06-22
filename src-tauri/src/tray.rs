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
pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_git = MenuItem::with_id(app, "show_git", "Git 设置", true, None::<&str>)?;
    let show_amkr = MenuItem::with_id(app, "show_amkr", "AMKR 设置", true, None::<&str>)?;
    let show_page_notes = MenuItem::with_id(app, "show_page_notes", "页面笔记设置", true, None::<&str>)?;
    let manage = MenuItem::with_id(app, "manage", "插件管理", true, None::<&str>)?;
    let hide_all = MenuItem::with_id(app, "hide_all", "隐藏全部", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_git, &show_amkr, &show_page_notes, &manage, &hide_all, &quit])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Galncelet")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show_git" => open_settings(&app, "git"),
            "show_amkr" => open_settings(&app, "amkr"),
            "show_page_notes" => open_settings(&app, "page-notes"),
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
