use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAddonManifest {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub default_width: Option<f64>,
    #[serde(default)]
    pub default_height: Option<f64>,
    #[serde(default)]
    pub show_close_button: Option<bool>,
    #[serde(default)]
    pub show_collapse_button: Option<bool>,
    #[serde(default)]
    pub show_attach_button: Option<bool>,
    #[serde(default)]
    pub default_attach_enabled: Option<bool>,
    #[serde(default)]
    pub default_attach_remember: Option<bool>,
    #[serde(default)]
    pub default_whitelist: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub backend: Option<RuntimeAddonBackend>,
    #[serde(skip)]
    pub root_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAddonBackend {
    #[serde(rename = "type")]
    pub backend_type: RuntimeAddonBackendType,
    pub command: String,
    #[serde(default = "default_backend_protocol")]
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeAddonBackendType {
    Sidecar,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAddonInfo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub entry: String,
    pub default_width: Option<f64>,
    pub default_height: Option<f64>,
    pub show_close_button: Option<bool>,
    pub show_collapse_button: Option<bool>,
    pub show_attach_button: Option<bool>,
    pub default_attach_enabled: Option<bool>,
    pub default_attach_remember: Option<bool>,
    pub default_whitelist: Vec<String>,
    pub permissions: Vec<String>,
    pub has_backend: bool,
}

fn default_entry() -> String {
    "ui/index.html".to_string()
}

fn default_backend_protocol() -> String {
    "jsonrpc".to_string()
}

fn is_safe_addon_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

fn validate_relative_file(root: &Path, relative: &str, label: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!("{label} must be a safe relative path"));
    }
    let full_path = root.join(relative_path);
    if !full_path.is_file() {
        return Err(format!("{label} not found: {}", full_path.display()));
    }
    Ok(full_path)
}

pub fn parse_manifest(root: &Path, data: &str) -> Result<RuntimeAddonManifest, String> {
    let mut manifest: RuntimeAddonManifest =
        serde_json::from_str(data).map_err(|e| format!("Failed to parse manifest: {e}"))?;

    if !is_safe_addon_id(&manifest.id) {
        return Err("Addon id must contain only lowercase letters, digits, '-' or '_'".to_string());
    }
    if manifest.title.trim().is_empty() {
        return Err("Addon title cannot be empty".to_string());
    }

    validate_relative_file(root, &manifest.entry, "entry")?;
    if let Some(backend) = &manifest.backend {
        if backend.protocol != "jsonrpc" {
            return Err("Only jsonrpc sidecar backend protocol is supported".to_string());
        }
        validate_relative_file(root, &backend.command, "backend.command")?;
    }

    manifest.root_dir = root.to_path_buf();
    Ok(manifest)
}

pub fn scan_addons_dir(addons_dir: &Path) -> Result<Vec<RuntimeAddonManifest>, String> {
    if !addons_dir.exists() {
        return Ok(Vec::new());
    }

    let mut manifests = Vec::new();
    let mut seen = HashSet::new();
    for entry in fs::read_dir(addons_dir).map_err(|e| format!("Failed to read addons dir: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read addon entry: {e}"))?;
        let root = entry.path();
        if !root.is_dir() {
            continue;
        }
        let manifest_path = root.join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        let data = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read {}: {e}", manifest_path.display()))?;
        let manifest = parse_manifest(&root, &data)?;
        if !seen.insert(manifest.id.clone()) {
            return Err(format!("Duplicate addon id: {}", manifest.id));
        }
        manifests.push(manifest);
    }
    manifests.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(manifests)
}

impl RuntimeAddonManifest {
    pub fn to_info(&self) -> RuntimeAddonInfo {
        RuntimeAddonInfo {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            icon: self.icon.clone(),
            entry: self.entry.clone(),
            default_width: self.default_width,
            default_height: self.default_height,
            show_close_button: self.show_close_button,
            show_collapse_button: self.show_collapse_button,
            show_attach_button: self.show_attach_button,
            default_attach_enabled: self.default_attach_enabled,
            default_attach_remember: self.default_attach_remember,
            default_whitelist: self.default_whitelist.clone(),
            permissions: self.permissions.clone(),
            has_backend: self.backend.is_some(),
        }
    }
}

pub fn addons_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?
        .join("addons");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create addons dir: {e}"))?;
    Ok(dir)
}

pub fn load_runtime_addons(app: &tauri::AppHandle) -> Result<Vec<RuntimeAddonManifest>, String> {
    scan_addons_dir(&addons_dir(app)?)
}

pub fn load_runtime_addon(
    app: &tauri::AppHandle,
    addon_id: &str,
) -> Result<RuntimeAddonManifest, String> {
    load_runtime_addons(app)?
        .into_iter()
        .find(|addon| addon.id == addon_id)
        .ok_or_else(|| format!("Runtime addon not found: {addon_id}"))
}

pub fn addon_entry_path(app: &tauri::AppHandle, addon_id: &str) -> Result<PathBuf, String> {
    let addon = load_runtime_addon(app, addon_id)?;
    validate_relative_file(&addon.root_dir, &addon.entry, "entry")
}

fn jsonrpc_request(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    })
}

pub fn invoke_sidecar_backend(
    app: &tauri::AppHandle,
    addon_id: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if method.trim().is_empty() {
        return Err("JSON-RPC method cannot be empty".to_string());
    }

    let addon = load_runtime_addon(app, addon_id)?;
    let backend = addon
        .backend
        .ok_or_else(|| format!("Runtime addon has no backend: {addon_id}"))?;
    let command = validate_relative_file(&addon.root_dir, &backend.command, "backend.command")?;
    let request = jsonrpc_request(method, params);

    let mut process = sidecar_command(&command)
        .current_dir(&addon.root_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start addon backend: {e}"))?;

    if let Some(stdin) = process.stdin.as_mut() {
        stdin
            .write_all(request.to_string().as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .map_err(|e| format!("Failed to write JSON-RPC request: {e}"))?;
    }

    let output = process
        .wait_with_output()
        .map_err(|e| format!("Failed to read addon backend response: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("Addon backend exited with status {}", output.status)
        } else {
            stderr
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Addon backend returned invalid JSON: {e}"))?;
    if let Some(error) = response.get("error") {
        return Err(error.to_string());
    }
    Ok(response
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

fn sidecar_command(command: &Path) -> Command {
    let mut cmd = Command::new(command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

fn addon_fingerprint(root: &Path) -> String {
    fn visit(path: &Path, count: &mut u64, max_modified: &mut u128) {
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            *count += 1;
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    *max_modified = (*max_modified).max(duration.as_nanos());
                }
            }
            if metadata.is_dir() {
                visit(&path, count, max_modified);
            }
        }
    }

    let mut count = 0;
    let mut max_modified = 0;
    visit(root, &mut count, &mut max_modified);
    format!("{count}:{max_modified}")
}

fn addon_fingerprints(addons: &[RuntimeAddonManifest]) -> HashMap<String, String> {
    addons
        .iter()
        .map(|addon| (addon.id.clone(), addon_fingerprint(&addon.root_dir)))
        .collect()
}

pub fn start_runtime_addon_watcher(app: tauri::AppHandle) {
    thread::spawn(move || {
        let initial_addons = load_runtime_addons(&app).unwrap_or_default();
        let mut known = addon_fingerprints(&initial_addons);

        loop {
            thread::sleep(Duration::from_secs(2));
            let addons = match load_runtime_addons(&app) {
                Ok(addons) => addons,
                Err(e) => {
                    eprintln!("[runtime-addons] watcher scan failed: {e}");
                    continue;
                }
            };
            let current = addon_fingerprints(&addons);
            if current == known {
                continue;
            }

            let known_ids: HashSet<String> = known.keys().cloned().collect();
            let current_ids: HashSet<String> = current.keys().cloned().collect();
            let removed_ids: Vec<String> = known_ids.difference(&current_ids).cloned().collect();
            let changed_ids: Vec<String> = current
                .iter()
                .filter_map(|(id, fingerprint)| {
                    known
                        .get(id)
                        .filter(|previous| *previous != fingerprint)
                        .map(|_| id.clone())
                })
                .collect();

            if !removed_ids.is_empty() || !changed_ids.is_empty() {
                for id in removed_ids.iter().chain(changed_ids.iter()) {
                    if let Some(win) = app.get_webview_window(&format!("runtime-addon-{id}")) {
                        let _ = win.destroy();
                    }
                }
                if !removed_ids.is_empty() {
                    if let Ok(mut settings) = crate::settings::load_settings(app.clone()) {
                        for removed_id in &removed_ids {
                            settings.panel_visibility.insert(removed_id.clone(), false);
                        }
                        let _ = crate::settings::save_settings(app.clone(), settings);
                    }
                }
            }

            let infos: Vec<RuntimeAddonInfo> =
                addons.iter().map(RuntimeAddonManifest::to_info).collect();
            let _ = app.emit("runtime-addons-changed", infos);
            known = current;
        }
    });
}

fn is_safe_storage_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn addon_storage_dir(app: &tauri::AppHandle, addon_id: &str) -> Result<PathBuf, String> {
    if !is_safe_addon_id(addon_id) {
        return Err("Invalid addon id".to_string());
    }
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?
        .join("addon-data")
        .join(addon_id);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create addon storage dir: {e}"))?;
    Ok(dir)
}

fn storage_value_path(
    app: &tauri::AppHandle,
    addon_id: &str,
    key: &str,
) -> Result<PathBuf, String> {
    if !is_safe_storage_key(key) {
        return Err("Storage key must contain only letters, digits, '-', '_' or '.' and be at most 128 chars".to_string());
    }
    Ok(addon_storage_dir(app, addon_id)?.join(format!("{key}.json")))
}

#[tauri::command]
pub fn runtime_addon_storage_get(
    app: tauri::AppHandle,
    addon_id: String,
    key: String,
) -> Result<serde_json::Value, String> {
    let path = storage_value_path(&app, &addon_id, &key)?;
    if !path.exists() {
        return Ok(serde_json::Value::Null);
    }
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read addon storage value: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse addon storage value: {e}"))
}

#[tauri::command]
pub fn runtime_addon_storage_set(
    app: tauri::AppHandle,
    addon_id: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let path = storage_value_path(&app, &addon_id, &key)?;
    let data = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("Failed to serialize addon storage value: {e}"))?;
    fs::write(path, data).map_err(|e| format!("Failed to write addon storage value: {e}"))
}

#[tauri::command]
pub fn runtime_addon_storage_delete(
    app: tauri::AppHandle,
    addon_id: String,
    key: String,
) -> Result<(), String> {
    let path = storage_value_path(&app, &addon_id, &key)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to delete addon storage value: {e}")),
    }
}

#[tauri::command]
pub fn invoke_runtime_addon(
    app: tauri::AppHandle,
    addon_id: String,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    invoke_sidecar_backend(&app, &addon_id, &method, params)
}

#[tauri::command]
pub fn list_runtime_addons(app: tauri::AppHandle) -> Result<Vec<RuntimeAddonInfo>, String> {
    Ok(load_runtime_addons(&app)?
        .into_iter()
        .map(|addon| addon.to_info())
        .collect())
}

#[tauri::command]
pub fn get_runtime_addons_dir(app: tauri::AppHandle) -> Result<String, String> {
    Ok(addons_dir(&app)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_runtime_addons_dir(app: tauri::AppHandle) -> Result<String, String> {
    let dir = addons_dir(&app)?;
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    }
    Ok(dir.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_addon_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("galncelet-{name}-{unique}"));
        fs::create_dir_all(root.join("ui")).unwrap();
        fs::write(root.join("ui/index.html"), "<h1>Hello</h1>").unwrap();
        root
    }

    #[test]
    fn parses_frontend_only_manifest() {
        let root = temp_addon_root("frontend");
        let manifest = parse_manifest(
            &root,
            r#"{
              "id": "hello-world",
              "title": "Hello World",
              "entry": "ui/index.html",
              "defaultWidth": 320,
              "defaultHeight": 180
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.id, "hello-world");
        assert_eq!(manifest.entry, "ui/index.html");
        assert_eq!(manifest.default_width, Some(320.0));
        assert!(manifest.backend.is_none());
    }

    #[test]
    fn rejects_manifest_with_path_traversal_entry() {
        let root = temp_addon_root("traversal");
        let err = parse_manifest(
            &root,
            r#"{"id":"bad","title":"Bad","entry":"../secret.html"}"#,
        )
        .unwrap_err();

        assert!(err.contains("safe relative path"));
    }

    #[test]
    fn rejects_unsafe_storage_keys() {
        assert!(is_safe_storage_key("settings.theme"));
        assert!(is_safe_storage_key("user_1-cache"));
        assert!(!is_safe_storage_key("../secret"));
        assert!(!is_safe_storage_key(""));
    }

    #[test]
    fn fingerprints_change_when_addon_files_change() {
        let root = temp_addon_root("fingerprint");
        let before = addon_fingerprint(&root);
        std::thread::sleep(std::time::Duration::from_millis(2));
        fs::write(root.join("ui/main.js"), "console.log('changed')").unwrap();

        let after = addon_fingerprint(&root);

        assert_ne!(before, after);
    }

    #[test]
    fn tracks_addon_ids() {
        let root = temp_addon_root("ids");
        let one = parse_manifest(&root, r#"{"id":"one","title":"One"}"#).unwrap();
        let two = parse_manifest(&root, r#"{"id":"two","title":"Two"}"#).unwrap();

        let fingerprints = addon_fingerprints(&[one, two]);

        assert!(fingerprints.contains_key("one"));
        assert!(fingerprints.contains_key("two"));
    }

    #[test]
    fn builds_jsonrpc_request() {
        let request = jsonrpc_request("ping", serde_json::json!({ "ok": true }));

        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["id"], 1);
        assert_eq!(request["method"], "ping");
        assert_eq!(request["params"]["ok"], true);
    }

    #[test]
    fn parses_sidecar_backend_manifest() {
        let root = temp_addon_root("sidecar");
        fs::create_dir_all(root.join("backend")).unwrap();
        fs::write(root.join("backend/hello.cmd"), "@echo off").unwrap();

        let manifest = parse_manifest(
            &root,
            r#"{
              "id": "hello-sidecar",
              "title": "Hello Sidecar",
              "entry": "ui/index.html",
              "backend": {
                "type": "sidecar",
                "command": "backend/hello.cmd",
                "protocol": "jsonrpc"
              }
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.backend.unwrap().command, "backend/hello.cmd");
    }
}
