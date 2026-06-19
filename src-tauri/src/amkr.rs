use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::git;

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

/// Resolve AMKR connection info from config.
fn resolve_amkr() -> Option<(String, u16, String)> {
    let path = config_path()?;
    if !path.exists() {
        return None;
    }
    let data = fs::read_to_string(&path).ok()?;
    let config: AmkrConfig = serde_json::from_str(&data).ok()?;
    let host = config.host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = config.port.unwrap_or(8000);
    let api_key = config.local_api_key.unwrap_or_default();
    Some((host, port, api_key))
}

/// Tauri command: generate a commit message from staged changes using AMKR.
/// Returns the generated message, or an error string.
#[tauri::command]
pub async fn generate_commit_message(repo_root: String) -> Result<String, String> {
    let (host, port, api_key) = resolve_amkr()
        .ok_or("AMKR 未安装或配置文件不存在")?;

    // Get staged diff
    let diff = git::get_staged_diff_summary(&repo_root)?;

    if diff.trim().is_empty() {
        return Err("没有暂存的更改".to_string());
    }

    let prompt = format!(
        "根据以下 git diff 生成一条符合 Conventional Commits 规范的 commit 信息。\n\n\
         规则：\n\
         - 格式: <type>(<scope>): <subject>\\n\\n<body>\n\
         - type 可选: feat, fix, docs, style, refactor, perf, test, build, ci, chore\n\
         - subject 用中文，首字母小写，不加句号\n\
         - body 用中文简要说明改动内容和原因\n\
         - 只输出 commit 信息本身，不要解释，不要代码块标记\n\n\
         ```\n{}\n```",
        diff
    );

    let body = serde_json::json!({
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3
    });

    let url = format!("http://{}:{}/v1/chat/completions", host, port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let mut req = client.post(&url)
        .header("Content-Type", "application/json")
        .body(body.to_string());
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = req.send().await
        .map_err(|e| format!("AMKR 请求失败: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("AMKR 返回 {}: {}", status, text));
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("解析 AMKR 响应失败: {e}"))?;

    // Extract message from OpenAI-compatible response
    let message = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if message.is_empty() {
        return Err("AI 未返回有效内容".to_string());
    }

    Ok(message)
}
