use std::fs;
use std::path::PathBuf;
use tauri::Manager;

const EXT_DIR_NAME: &str = "browser-extension";

/// Resolve the path to the browser extension in app data (persistent).
/// Copies from resources on first run.
fn ext_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let ext_dir = data_dir.join(EXT_DIR_NAME);

    if !ext_dir.join("manifest.json").exists() {
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to resolve resource dir: {e}"))?;
        let src = resource_dir.join(EXT_DIR_NAME);
        if src.exists() {
            copy_dir_recursive(&src, &ext_dir)
                .map_err(|e| format!("Failed to copy extension: {e}"))?;
            println!("[browser-ext] Copied extension to {:?}", ext_dir);
        } else {
            let dev_src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("src/addons/page-notes")
                .join(EXT_DIR_NAME);
            if dev_src.exists() {
                copy_dir_recursive(&dev_src, &ext_dir)
                    .map_err(|e| format!("Failed to copy extension: {e}"))?;
                println!("[browser-ext] Copied extension (dev) to {:?}", ext_dir);
            } else {
                return Err(format!(
                    "Extension not found at {:?} or {:?}",
                    src, dev_src
                ));
            }
        }
    }

    Ok(ext_dir)
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Setup: copy extension to app data.
pub fn setup(app: &tauri::AppHandle) {
    match ext_data_dir(app) {
        Ok(ext_dir) => {
            println!("[browser-ext] Extension ready at {:?}", ext_dir);
        }
        Err(e) => eprintln!("[browser-ext] Setup failed: {}", e),
    }
}

#[cfg(target_os = "windows")]
fn setup_registry(ext_path: &str) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey("Software\\Galncelet")
        .map_err(|e| format!("Failed to create registry key: {e}"))?;
    key.set_value("WsPort", &"17233")
        .map_err(|e| format!("Failed to set WsPort: {e}"))?;
    key.set_value("ExtensionPath", &ext_path)
        .map_err(|e| format!("Failed to set ExtensionPath: {e}"))?;
    Ok(())
}

// ─── Tauri commands ────────────────────────────────────────────────

/// Tauri command: open the extension directory in Explorer.
#[tauri::command]
pub fn open_extension_dir(app: tauri::AppHandle) -> Result<String, String> {
    let ext_dir = ext_data_dir(&app)?;
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(ext_dir.to_string_lossy().to_string())
            .spawn();
    }
    Ok(ext_dir.to_string_lossy().to_string())
}

/// Check if a process is running by image name.
#[cfg(target_os = "windows")]
fn is_process_running(name: &str) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", name), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(&name.to_lowercase()))
        .unwrap_or(false)
}

/// Tauri command: launch Chrome/Edge with the extension loaded.
/// Returns an error if the browser is already running (user must close it first).
#[tauri::command]
pub fn launch_browser_with_extension(app: tauri::AppHandle) -> Result<String, String> {
    let ext_dir = ext_data_dir(&app)?;
    let ext_path = ext_dir.to_string_lossy();

    // Check if Chrome or Edge is already running — --load-extension won't work
    if is_process_running("chrome.exe") {
        return Err("Chrome 已在运行，请先关闭 Chrome 再点击启动".to_string());
    }
    if is_process_running("msedge.exe") {
        return Err("Edge 已在运行，请先关闭 Edge 再点击启动".to_string());
    }

    let browsers: &[(&str, &str)] = &[
        (r"C:\Program Files\Google\Chrome\Application\chrome.exe", "Chrome"),
        (r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe", "Chrome"),
        (r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe", "Edge"),
        (r"C:\Program Files\Microsoft\Edge\Application\msedge.exe", "Edge"),
    ];

    for (path, name) in browsers {
        if std::path::Path::new(path).exists() {
            std::process::Command::new(path)
                .arg(format!("--load-extension={}", ext_path))
                .spawn()
                .map_err(|e| format!("Failed to start {}: {}", name, e))?;
            return Ok(name.to_string());
        }
    }

    Err("未找到 Chrome 或 Edge，请安装其中一种浏览器".to_string())
}

