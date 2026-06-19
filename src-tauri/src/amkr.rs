use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Relevant fields from AMKR's router-config.json.
#[derive(Debug, Deserialize)]
struct AmkrConfig {
    host: Option<String>,
    port: Option<u16>,
    local_api_key: Option<String>,
}

/// Resolve the default AMKR config file path.
/// On Windows: %LOCALAPPDATA%/AutoModelKeyRouter/router-config.json
fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("APPDATA"))
            .ok()?;
        Some(PathBuf::from(base).join("AutoModelKeyRouter").join("router-config.json"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".cache").join("auto-model-key-router").join("router-config.json"))
    }
}

/// Tauri command: fetch metrics from a running AMKR instance.
/// Returns the raw JSON on success, or `None` if AMKR is not installed or not reachable.
#[tauri::command]
pub async fn fetch_amkr_metrics() -> Result<Option<serde_json::Value>, String> {
    // Read AMKR config
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(None),
    };

    if !path.exists() {
        return Ok(None);
    }

    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Ok(None),
    };

    let config: AmkrConfig = match serde_json::from_str(&data) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let host = config.host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = config.port.unwrap_or(8000);
    let api_key = config.local_api_key.unwrap_or_default();

    let url = format!("http://{}:{}/metrics?hours=24", host, port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let mut req = client.get(&url);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(_) => return Ok(None), // AMKR not reachable — silent fail
    };

    if !resp.status().is_success() {
        return Ok(None);
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse AMKR response: {e}"))?;

    Ok(Some(json))
}
