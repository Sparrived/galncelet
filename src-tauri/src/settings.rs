use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
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
    /// Whether `git pull` uses --rebase (default true)
    pub pull_rebase: bool,
    /// Saved repository paths
    pub saved_repos: Vec<String>,
    /// Panel visibility keyed by panel id
    pub panel_visibility: HashMap<String, bool>,
    /// Per-window persisted state (position, attach, etc.)
    pub window_states: HashMap<String, WindowState>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut panel_visibility = HashMap::new();
        panel_visibility.insert("git".to_string(), true);
        panel_visibility.insert("amkr".to_string(), true);
        panel_visibility.insert("page-notes".to_string(), true);
        Self {
            refresh_interval_ms: 2000,
            card_width: 360,
            log_max_count: 50,
            always_on_top: true,
            pull_rebase: true,
            saved_repos: Vec::new(),
            panel_visibility,
            window_states: HashMap::new(),
        }
    }
}

/// Resolve the path to `settings.json` inside the app data directory.
fn settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create app data dir: {e}"))?;
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
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read settings: {e}"))?;
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
