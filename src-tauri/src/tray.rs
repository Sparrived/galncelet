use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// Set up the system tray icon with context menu.
pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_git = MenuItem::with_id(app, "show_git", "显示 Git 挂件", true, None::<&str>)?;
    let show_amkr = MenuItem::with_id(app, "show_amkr", "显示 AMKR 挂件", true, None::<&str>)?;
    let show_page_notes = MenuItem::with_id(app, "show_page_notes", "显示页面笔记", true, None::<&str>)?;
    let manage = MenuItem::with_id(app, "manage", "插件管理", true, None::<&str>)?;
    let hide_all = MenuItem::with_id(app, "hide_all", "隐藏全部", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_git, &show_amkr, &show_page_notes, &manage, &hide_all, &quit])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Galncelet")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show_git" => {
                if let Some(win) = app.get_webview_window("widget-git") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "show_amkr" => {
                if let Some(win) = app.get_webview_window("widget-amkr") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "show_page_notes" => {
                if let Some(win) = app.get_webview_window("widget-page-notes") {
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
        })
        .on_tray_icon_event(|_tray, _event| {
            // Left-click: no action (use right-click menu to manage widgets)
        })
        .build(app)?;

    Ok(())
}
