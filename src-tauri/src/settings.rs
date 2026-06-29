use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::Manager;

/// Per-window persisted state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    /// Logical X position (None = use default)
    pub x: Option<f64>,
    /// Logical Y position (None = use default)
    pub y: Option<f64>,
    /// Logical height (None = use plugin default)
    pub height: Option<f64>,
    /// Whether attach-to-foreground is enabled (None = use plugin default)
    pub attach_enabled: Option<bool>,
    /// Attach whitelist — foreground window title substrings
    pub whitelist: Option<Vec<String>>,
    /// When true, attach only manages show/hide, not position
    pub attach_remember: Option<bool>,
}

/// Application settings persisted as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Auto-refresh interval in milliseconds (default 2000)
    pub refresh_interval_ms: u32,
    /// Card window width in logical pixels (default 360)
    pub card_width: u32,
    /// Maximum number of log entries to display (default 50)
    pub log_max_count: u32,
    /// Whether the window stays on top (default true)
    pub always_on_top: bool,
    /// Whether the app starts with Windows login (default false)
    #[serde(default)]
    pub start_on_boot: bool,
    /// Whether `git pull` uses --rebase (default true)
    pub pull_rebase: bool,
    /// Saved repository paths
    pub saved_repos: Vec<String>,
    /// Panel visibility keyed by panel id
    pub panel_visibility: HashMap<String, bool>,
    /// Per-window persisted state (position, attach, etc.)
    pub window_states: HashMap<String, WindowState>,
    /// Plugin hotkeys: pluginId → shortcut string (e.g. "Ctrl+Shift+1")
    #[serde(default)]
    pub plugin_hotkeys: HashMap<String, String>,
    /// Widget sequence: ordered plugin IDs that share the same position
    #[serde(default)]
    pub widget_sequence: Vec<String>,
    /// Hotkey to cycle through the widget sequence
    #[serde(default)]
    pub sequence_hotkey: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 2000,
            card_width: 360,
            log_max_count: 50,
            always_on_top: true,
            start_on_boot: false,
            pull_rebase: true,
            saved_repos: Vec::new(),
            panel_visibility: HashMap::new(),
            window_states: HashMap::new(),
            plugin_hotkeys: HashMap::new(),
            widget_sequence: Vec::new(),
            sequence_hotkey: None,
        }
    }
}

impl AppSettings {
    /// Ensure all known plugins have a visibility entry (defaulting to hidden).
    /// This fills in missing plugin IDs from manifests without overwriting existing values.
    pub fn ensure_plugin_visibility(&mut self, manifests: &[crate::plugins::PluginManifest]) {
        for m in manifests {
            self.panel_visibility.entry(m.id.clone()).or_insert(false);
        }
    }
}

/// Resolve the path to `settings.json` inside the app data directory.
fn settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(data_dir.join("settings.json"))
}

/// Tauri command: load application settings from disk.
/// Returns default settings if the file does not exist.
#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("Failed to read settings: {e}"))?;
    let settings: AppSettings =
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse settings: {e}"))?;
    Ok(settings)
}

/// Tauri command: save application settings to disk.
#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let data =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("Failed to serialize: {e}"))?;
    fs::write(&path, data).map_err(|e| format!("Failed to write settings: {e}"))?;
    Ok(())
}

#[cfg(windows)]
fn quote_windows_path(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

#[cfg(windows)]
fn escape_single_quoted_powershell(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn find_dev_project_root(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .find(|path| path.join("package.json").exists() && path.join("src-tauri").exists())
        .map(Path::to_path_buf)
}

#[cfg(windows)]
fn dev_startup_powershell(project_root: &Path, exe: &Path) -> String {
    let root = escape_single_quoted_powershell(&project_root.display().to_string());
    let exe = escape_single_quoted_powershell(&exe.display().to_string());

    format!(
        r#"$ErrorActionPreference = 'SilentlyContinue'
$projectRoot = '{root}'
$appExe = '{exe}'
$devUrl = 'http://127.0.0.1:1420'

function Test-FrontendReady {{
    try {{
        Invoke-WebRequest -UseBasicParsing -Uri $devUrl -TimeoutSec 1 | Out-Null
        return $true
    }} catch {{
        return $false
    }}
}}

if (-not (Test-FrontendReady)) {{
    Start-Process -FilePath 'npm.cmd' -ArgumentList @('run', 'dev', '--', '--host', '127.0.0.1') -WorkingDirectory $projectRoot -WindowStyle Hidden
}}

for ($i = 0; $i -lt 60; $i++) {{
    if (Test-FrontendReady) {{ break }}
    Start-Sleep -Milliseconds 500
}}

Start-Process -FilePath $appExe -WorkingDirectory $projectRoot -WindowStyle Hidden
"#
    )
}

#[cfg(windows)]
fn dev_startup_vbs(ps1_path: &Path) -> String {
    let ps1 = ps1_path.display().to_string().replace('"', "\"\"");
    format!(
        "Set shell = CreateObject(\"WScript.Shell\")\r\nshell.Run \"powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File \"\"{ps1}\"\"\", 0, False\r\n"
    )
}

#[cfg(windows)]
fn write_dev_startup_launcher(app: &tauri::AppHandle, exe: &Path) -> Result<PathBuf, String> {
    let project_root = find_dev_project_root(exe).ok_or_else(|| {
        format!(
            "Failed to find project root from executable: {}",
            exe.display()
        )
    })?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;

    let ps1_path = data_dir.join("startup-dev.ps1");
    let vbs_path = data_dir.join("startup-dev.vbs");
    fs::write(&ps1_path, dev_startup_powershell(&project_root, exe))
        .map_err(|e| format!("Failed to write dev startup PowerShell launcher: {e}"))?;
    fs::write(&vbs_path, dev_startup_vbs(&ps1_path))
        .map_err(|e| format!("Failed to write dev startup VBScript launcher: {e}"))?;
    Ok(vbs_path)
}

#[cfg(windows)]
fn startup_registry_command(app: &tauri::AppHandle, exe: &Path) -> Result<String, String> {
    if cfg!(debug_assertions) {
        let launcher = write_dev_startup_launcher(app, exe)?;
        Ok(format!(
            "wscript.exe //B //NoLogo {}",
            quote_windows_path(&launcher)
        ))
    } else {
        Ok(quote_windows_path(exe))
    }
}

/// Tauri command: enable or disable Windows login startup.
#[tauri::command]
pub fn set_start_on_boot(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run_key, _) = hkcu
            .create_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Run",
                KEY_READ | KEY_WRITE,
            )
            .map_err(|e| format!("Failed to open Windows startup registry key: {e}"))?;

        let app_name = app.package_info().name.clone();
        if enabled {
            let exe = env::current_exe()
                .map_err(|e| format!("Failed to resolve current executable: {e}"))?;
            let command = startup_registry_command(&app, &exe)?;
            run_key
                .set_value(&app_name, &command)
                .map_err(|e| format!("Failed to enable startup: {e}"))?;
            let stored: String = run_key
                .get_value(&app_name)
                .map_err(|e| format!("Failed to verify startup registry value: {e}"))?;
            if stored != command {
                return Err(format!(
                    "Startup registry verification failed: expected {command}, got {stored}"
                ));
            }
        } else {
            match run_key.delete_value(&app_name) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Failed to disable startup: {e}")),
            }
            match run_key.get_value::<String, _>(&app_name) {
                Ok(value) => {
                    return Err(format!(
                        "Startup registry verification failed: value still exists as {value}"
                    ));
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Failed to verify startup registry removal: {e}")),
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = enabled;
    }

    let mut settings = load_settings(app.clone())?;
    settings.start_on_boot = enabled;
    save_settings(app, settings)?;

    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn finds_project_root_from_debug_exe_path() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let exe = root
            .join("src-tauri")
            .join("target")
            .join("debug")
            .join("galncelet.exe");

        assert_eq!(find_dev_project_root(&exe), Some(root));
    }

    #[test]
    fn dev_launcher_starts_frontend_hidden_before_app() {
        let root = PathBuf::from(r"D:\Code\galncelet");
        let exe = root
            .join("src-tauri")
            .join("target")
            .join("debug")
            .join("galncelet.exe");

        let script = dev_startup_powershell(&root, &exe);

        assert!(script.contains("Start-Process -FilePath 'npm.cmd'"));
        assert!(script.contains("-WindowStyle Hidden"));
        assert!(script.contains("'--host', '127.0.0.1'"));
        assert!(
            script.find("Start-Process -FilePath 'npm.cmd'").unwrap()
                < script.find("Start-Process -FilePath $appExe").unwrap()
        );
    }

    #[test]
    fn vbs_launcher_runs_powershell_without_a_console_window() {
        let ps1 = PathBuf::from(r"C:\Users\Me\AppData\Roaming\Galncelet\startup-dev.ps1");

        let script = dev_startup_vbs(&ps1);

        assert!(script
            .contains("powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden"));
        assert!(script.contains(", 0, False"));
    }
}

/// Tauri command: set plugin visibility in settings (used by close button).
#[tauri::command]
pub fn set_plugin_visible(
    app: tauri::AppHandle,
    plugin_id: String,
    visible: bool,
) -> Result<(), String> {
    let mut s = load_settings(app.clone())?;
    s.panel_visibility.insert(plugin_id, visible);
    save_settings(app, s)
}

/// Tauri command: update a single window's persisted state.
/// Reads the current settings, merges the window state, and writes back.
#[tauri::command]
pub fn save_window_state(
    app: tauri::AppHandle,
    window_id: String,
    state: WindowState,
) -> Result<(), String> {
    let mut settings = load_settings(app.clone())?;
    settings.window_states.insert(window_id, state);
    save_settings(app, settings)
}

/// Tauri command: set or clear a plugin's global hotkey.
#[tauri::command]
pub fn set_plugin_hotkey(
    app: tauri::AppHandle,
    plugin_id: String,
    hotkey: Option<String>,
) -> Result<(), String> {
    let mut settings = load_settings(app.clone())?;
    if let Some(hk) = hotkey {
        settings.plugin_hotkeys.insert(plugin_id, hk);
    } else {
        settings.plugin_hotkeys.remove(&plugin_id);
    }
    save_settings(app.clone(), settings.clone())?;
    // Re-register all hotkeys
    unregister_all_hotkeys(&app);
    register_all_hotkeys(&app, &settings);
    Ok(())
}

/// Tauri command: set the widget sequence order.
/// Only hides/shows windows and does not change panel_visibility.
#[tauri::command]
pub fn set_widget_sequence(app: tauri::AppHandle, sequence: Vec<String>) -> Result<(), String> {
    let mut settings = load_settings(app.clone())?;
    settings.widget_sequence = sequence.clone();
    save_settings(app.clone(), settings.clone())?;

    // Sync sequence labels with attach state so the attach loop skips them
    if let Some(state) = app.try_state::<std::sync::Arc<crate::window_attach::AttachState>>() {
        let mut sl = state.sequence_labels.lock().unwrap();
        sl.clear();
        for pid in &sequence {
            sl.insert(format!("widget-{}", pid));
        }
    }

    // Hide all sequence windows except the first one
    SEQUENCE_INDEX.store(0, Ordering::Relaxed);
    for (i, pid) in sequence.iter().enumerate() {
        let label = format!("widget-{}", pid);
        if let Some(win) = app.get_webview_window(&label) {
            if i == 0 {
                let _ = win.show();
            } else {
                let _ = win.hide();
            }
        }
    }
    unregister_all_hotkeys(&app);
    register_all_hotkeys(&app, &settings);
    Ok(())
}

/// Tauri command: set or clear the sequence cycle hotkey.
#[tauri::command]
pub fn set_sequence_hotkey(app: tauri::AppHandle, hotkey: Option<String>) -> Result<(), String> {
    let mut settings = load_settings(app.clone())?;
    settings.sequence_hotkey = hotkey;
    save_settings(app.clone(), settings.clone())?;
    unregister_all_hotkeys(&app);
    register_all_hotkeys(&app, &settings);
    Ok(())
}

/// Parse a shortcut string like "Ctrl+Shift+1" and register it.
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn parse_shortcut(s: &str) -> Option<Shortcut> {
    let mut mods = Modifiers::empty();
    let mut code_str = String::new();
    for part in s.split('+') {
        let p = part.trim().to_lowercase();
        match p.as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "super" | "win" | "meta" | "cmd" | "command" => mods |= Modifiers::SUPER,
            other => code_str = other.to_string(),
        }
    }
    let code = match code_str.as_str() {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        "space" => Code::Space,
        "tab" => Code::Tab,
        "enter" => Code::Enter,
        "escape" | "esc" => Code::Escape,
        "backspace" => Code::Backspace,
        "slash" => Code::Slash,
        "backslash" => Code::Backslash,
        "period" | "." => Code::Period,
        "comma" | "," => Code::Comma,
        "semicolon" | ";" => Code::Semicolon,
        "quote" | "'" => Code::Quote,
        "bracketleft" | "[" => Code::BracketLeft,
        "bracketright" | "]" => Code::BracketRight,
        "minus" | "-" => Code::Minus,
        "equal" | "=" => Code::Equal,
        "backquote" | "`" => Code::Backquote,
        "up" => Code::ArrowUp,
        "down" => Code::ArrowDown,
        "left" => Code::ArrowLeft,
        "right" => Code::ArrowRight,
        "delete" => Code::Delete,
        "insert" => Code::Insert,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        "numpad0" => Code::Numpad0,
        "numpad1" => Code::Numpad1,
        "numpad2" => Code::Numpad2,
        "numpad3" => Code::Numpad3,
        "numpad4" => Code::Numpad4,
        "numpad5" => Code::Numpad5,
        "numpad6" => Code::Numpad6,
        "numpad7" => Code::Numpad7,
        "numpad8" => Code::Numpad8,
        "numpad9" => Code::Numpad9,
        _ => return None,
    };
    Some(Shortcut::new(Some(mods), code))
}

/// Register all plugin hotkeys from settings.
pub fn register_all_hotkeys(app: &tauri::AppHandle, settings: &AppSettings) {
    // Register per-plugin hotkeys
    for (plugin_id, hotkey_str) in &settings.plugin_hotkeys {
        let shortcut = match parse_shortcut(hotkey_str) {
            Some(s) => s,
            None => {
                eprintln!(
                    "[hotkey] invalid shortcut for {}: {}",
                    plugin_id, hotkey_str
                );
                continue;
            }
        };
        let pid = plugin_id.clone();
        let app_handle = app.clone();
        if let Err(e) =
            app.global_shortcut()
                .on_shortcut(shortcut, move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_plugin_visibility(&app_handle, &pid);
                    }
                })
        {
            eprintln!(
                "[hotkey] failed to register {} for {}: {}",
                hotkey_str, plugin_id, e
            );
        } else {
            println!("[hotkey] registered {} → {}", hotkey_str, plugin_id);
        }
    }

    // Register sequence cycle hotkey
    if let Some(ref hk) = settings.sequence_hotkey {
        if !settings.widget_sequence.is_empty() {
            let shortcut = match parse_shortcut(hk) {
                Some(s) => s,
                None => {
                    eprintln!("[hotkey] invalid sequence shortcut: {}", hk);
                    return;
                }
            };
            let app_handle = app.clone();
            if let Err(e) =
                app.global_shortcut()
                    .on_shortcut(shortcut, move |_app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            cycle_sequence(&app_handle);
                        }
                    })
            {
                eprintln!(
                    "[hotkey] failed to register sequence shortcut {}: {}",
                    hk, e
                );
            } else {
                println!(
                    "[hotkey] registered sequence {} ({} plugins)",
                    hk,
                    settings.widget_sequence.len()
                );
            }
        }
    }
}

/// Unregister all global shortcuts.
pub fn unregister_all_hotkeys(app: &tauri::AppHandle) {
    if let Err(e) = app.global_shortcut().unregister_all() {
        eprintln!("[hotkey] failed to unregister all: {}", e);
    }
}

/// Toggle a plugin's visibility: if hidden → show, if visible → hide.
fn toggle_plugin_visibility(app: &tauri::AppHandle, plugin_id: &str) {
    let is_in_sequence = load_settings(app.clone())
        .map(|settings| settings.widget_sequence.iter().any(|id| id == plugin_id))
        .unwrap_or(false);
    let label = format!("widget-{}", plugin_id);
    if let Some(win) = app.get_webview_window(&label) {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
            if !is_in_sequence {
                let _ = set_plugin_visible(app.clone(), plugin_id.to_string(), false);
            }
            println!("[hotkey] hidden {}", plugin_id);
        } else {
            let _ = win.show();
            if !is_in_sequence {
                let _ = set_plugin_visible(app.clone(), plugin_id.to_string(), true);
            }
            println!("[hotkey] shown {}", plugin_id);
        }
    } else {
        println!(
            "[hotkey] widget window {} not found, open via manage page",
            plugin_id
        );
    }
}

/// Global sequence index (which plugin in the sequence is currently active).
static SEQUENCE_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Cycle to the next widget in the sequence.
/// Gets position from the currently visible widget, hides it, shows the next one there.
fn cycle_sequence(app: &tauri::AppHandle) {
    let settings = match load_settings(app.clone()) {
        Ok(s) => s,
        Err(_) => return,
    };
    let seq = &settings.widget_sequence;
    if seq.is_empty() {
        return;
    }

    let idx = SEQUENCE_INDEX.load(Ordering::Relaxed);
    let safe_idx = idx % seq.len();
    let current_id = &seq[safe_idx];

    // Get position from the currently visible widget (the one being replaced)
    let current_label = format!("widget-{}", current_id);
    let pos: Option<(i32, i32)> = app
        .get_webview_window(&current_label)
        .and_then(|win| win.outer_position().ok())
        .map(|p| (p.x, p.y));

    // Hide current widget (window only, plugin stays enabled)
    if let Some(win) = app.get_webview_window(&current_label) {
        let _ = win.hide();
    }

    // Advance index
    let next_idx = (safe_idx + 1) % seq.len();
    SEQUENCE_INDEX.store(next_idx, Ordering::Relaxed);

    // Show next widget at the same position; skip if window doesn't exist
    let mut tried = 0;
    let mut idx_to_show = next_idx;
    while tried < seq.len() {
        let id = &seq[idx_to_show];
        let label = format!("widget-{}", id);
        if let Some(win) = app.get_webview_window(&label) {
            if let Some((x, y)) = pos {
                let _ =
                    win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
            }
            let _ = win.show();
            let _ = win.set_focus();
            SEQUENCE_INDEX.store(idx_to_show, Ordering::Relaxed);
            println!(
                "[sequence] switched to {} ({}/{})",
                id,
                idx_to_show + 1,
                seq.len()
            );
            return;
        }
        println!("[sequence] widget {} not found, skipping", id);
        idx_to_show = (idx_to_show + 1) % seq.len();
        tried += 1;
    }
    println!("[sequence] no available widgets in sequence");
}
