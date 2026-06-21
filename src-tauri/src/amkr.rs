use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::git;

/// Relevant fields from AMKR's router-config.json.
#[derive(Debug, Deserialize)]
struct AmkrConfig {
    host: Option<String>,
    port: Option<u16>,
    local_api_key: Option<String>,
    models: Option<Vec<AmkrModel>>,
    unified_model: Option<AmkrUnifiedModel>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AmkrModel {
    id: String,
    aliases: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AmkrUnifiedModel {
    model: String,
    key: Option<String>,
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

    let url = format!("http://{}:{}/metrics", host, port);

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

/// Tauri command: generate a commit message from staged changes using AMKR with streaming.
/// Returns the generated message, or an error string.
#[tauri::command]
pub async fn generate_commit_message(app: tauri::AppHandle, repo_root: String) -> Result<String, String> {
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
        "model": "unified-model",
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3,
        "stream": true
    });

    let url = format!("http://{}:{}/v1/chat/completions", host, port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
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

    // Process streaming response
    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut message = String::new();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("流式读取失败: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    continue;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        message.push_str(content);
                        // Emit progress event
                        let _ = app.emit("ai-commit-progress", &message);
                    }
                }
            }
        }
    }

    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("AI 未返回有效内容".to_string());
    }

    Ok(message)
}

/// Response structure for the models list API.
#[derive(Debug, Serialize)]
pub struct AmkrModelInfo {
    pub id: String,
    pub aliases: Vec<String>,
    pub is_current: bool,
}

/// Tauri command: get available models and current unified model selection.
#[tauri::command]
pub async fn get_amkr_models() -> Result<Option<Vec<AmkrModelInfo>>, String> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(None),
    };

    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path)
        .map_err(|e| format!("读取配置失败: {e}"))?;

    let config: AmkrConfig = serde_json::from_str(&data)
        .map_err(|e| format!("解析配置失败: {e}"))?;

    let models = config.models.unwrap_or_default();
    let current_model = config.unified_model.map(|u| u.model);

    let result: Vec<AmkrModelInfo> = models.iter().map(|m| {
        AmkrModelInfo {
            id: m.id.clone(),
            aliases: m.aliases.clone().unwrap_or_default(),
            is_current: current_model.as_ref() == Some(&m.id),
        }
    }).collect();

    Ok(Some(result))
}

/// Tauri command: update the unified model selection.
#[tauri::command]
pub async fn set_amkr_unified_model(model_id: String) -> Result<(), String> {
    let path = match config_path() {
        Some(p) => p,
        None => return Err("AMKR 配置文件路径不存在".to_string()),
    };

    if !path.exists() {
        return Err("AMKR 配置文件不存在".to_string());
    }

    let data = fs::read_to_string(&path)
        .map_err(|e| format!("读取配置失败: {e}"))?;

    let mut config: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| format!("解析配置失败: {e}"))?;

    // Update unified_model field
    config["unified_model"] = serde_json::json!({
        "model": model_id
    });

    // Write back to file
    let updated = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {e}"))?;

    fs::write(&path, updated)
        .map_err(|e| format!("写入配置失败: {e}"))?;

    Ok(())
}

/// Shared state for the AMKR WebSocket connection.
pub type AmkrWsHandle = Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>;

/// Tauri command: start the AMKR event WebSocket client.
/// Connects to `ws://{host}:{port}/ws/events`, authenticates, and forwards
/// events to the frontend via `amkr-event` Tauri events.
#[tauri::command]
pub async fn start_amkr_ws(
    app: tauri::AppHandle,
    state: tauri::State<'_, AmkrWsHandle>,
) -> Result<(), String> {
    // Stop any existing connection
    let mut handle = state.lock().await;
    if let Some(h) = handle.take() {
        h.abort();
    }

    let (host, port, api_key) = resolve_amkr()
        .ok_or("AMKR 未安装或配置文件不存在")?;

    let url = format!("ws://{}:{}/ws/events", host, port);

    let task = tokio::spawn(async move {
        let mut retry_delay = 1u64;
        loop {
            println!("[amkr] connecting to {}", url);
            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    retry_delay = 1;
                    let (mut write, mut read) = ws_stream.split();

                    // Send auth message
                    let auth = serde_json::json!({
                        "type": "auth",
                        "token": api_key,
                    });
                    if let Err(e) = write.send(Message::Text(auth.to_string().into())).await {
                        eprintln!("[amkr] failed to send auth: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(retry_delay)).await;
                        continue;
                    }

                    // Read events loop
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                match serde_json::from_str::<serde_json::Value>(&text) {
                                    Ok(event) => {
                                        let _ = app.emit("amkr-event", &event);
                                    }
                                    Err(e) => {
                                        eprintln!("[amkr] invalid event JSON: {}", e);
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Ok(Message::Ping(data)) => {
                                let _ = write.send(Message::Pong(data)).await;
                            }
                            Err(_) => break,
                            _ => {}
                        }
                    }
                    println!("[amkr] WebSocket disconnected, reconnecting...");
                }
                Err(e) => {
                    eprintln!("[amkr] connection failed: {}", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(retry_delay)).await;
            retry_delay = (retry_delay * 2).min(30);
        }
    });

    *handle = Some(task);
    Ok(())
}

/// Tauri command: stop the AMKR event WebSocket client.
#[tauri::command]
pub async fn stop_amkr_ws(
    state: tauri::State<'_, AmkrWsHandle>,
) -> Result<(), String> {
    let mut handle = state.lock().await;
    if let Some(h) = handle.take() {
        h.abort();
    }
    Ok(())
}
