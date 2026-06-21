use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use futures_util::{SinkExt, StreamExt};

/// URL matching mode for a rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    /// URL contains this substring
    Substring,
    /// URL matches this regex pattern
    Regex,
}

/// A single page-note rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNoteRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub match_mode: MatchMode,
    pub note: String,
    pub enabled: bool,
}

/// Top-level configuration for page notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNotesConfig {
    pub rules: Vec<PageNoteRule>,
    /// WebSocket server port (default 17233)
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,
}

fn default_ws_port() -> u16 {
    17233
}

impl Default for PageNotesConfig {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            ws_port: default_ws_port(),
        }
    }
}

/// Message received from the browser extension via WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsInMessage {
    UrlChange { url: String, title: String },
    Ping,
}

/// Message sent to the browser extension via WebSocket.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsOutMessage {
    NoteMatch { rule_id: String, name: String, note: String },
    NoMatch,
    Pong,
}

/// Payload emitted to the Tauri frontend as a `page-notes-update` event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNotesUpdate {
    pub url: String,
    pub title: String,
    pub matched: bool,
    pub rule_id: Option<String>,
    pub name: Option<String>,
    pub note: Option<String>,
}

/// Resolve the path to `page-notes.json` inside the app data directory.
fn config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(data_dir.join("page-notes.json"))
}

/// Tauri command: load page notes config from disk.
#[tauri::command]
pub fn load_page_notes(app: tauri::AppHandle) -> Result<PageNotesConfig, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        return Ok(PageNotesConfig::default());
    }
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read page notes config: {e}"))?;
    let config: PageNotesConfig =
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse page notes config: {e}"))?;
    Ok(config)
}

/// Tauri command: save page notes config to disk.
#[tauri::command]
pub fn save_page_notes(app: tauri::AppHandle, config: PageNotesConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    let data = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize page notes config: {e}"))?;
    fs::write(&path, data).map_err(|e| format!("Failed to write page notes config: {e}"))?;
    Ok(())
}

/// Tauri command: get the WebSocket server port.
#[tauri::command]
pub fn get_ws_port(app: tauri::AppHandle) -> Result<u16, String> {
    let config = load_page_notes(app)?;
    Ok(config.ws_port)
}

/// Match a URL against the configured rules.
/// Returns the first matching enabled rule (by order in the list).
fn find_matching_rule<'a>(url: &str, rules: &'a [PageNoteRule]) -> Option<&'a PageNoteRule> {
    rules.iter().find(|rule| {
        if !rule.enabled {
            return false;
        }
        match rule.match_mode {
            MatchMode::Substring => url.contains(&rule.pattern),
            MatchMode::Regex => {
                regex::Regex::new(&rule.pattern)
                    .ok()
                    .map_or(false, |re| re.is_match(url))
            }
        }
    })
}

/// Reload the config from disk (shared helper).
fn reload_config(app: &tauri::AppHandle) -> PageNotesConfig {
    load_page_notes(app.clone()).unwrap_or_default()
}

/// Start the WebSocket server for browser extension communication.
/// Runs in a background tokio task. Accepts connections on `127.0.0.1:port`.
pub async fn start_ws_server(app_handle: tauri::AppHandle) -> Result<(), String> {
    let config = reload_config(&app_handle);
    let port = config.ws_port;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("Failed to bind WebSocket server on port {}: {}", port, e))?;

    println!("[page-notes] WebSocket server listening on 127.0.0.1:{}", port);

    // Shared current URL state for the Tauri command interface
    let current_state: Arc<Mutex<Option<PageNotesUpdate>>> = Arc::new(Mutex::new(None));

    tokio::spawn(async move {
        while let Ok((stream, _addr)) = listener.accept().await {
            let app = app_handle.clone();
            let state = current_state.clone();

            tokio::spawn(async move {
                if let Ok(ws_stream) = accept_async(stream).await {
                    let (mut write, mut read) = ws_stream.split();
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                match serde_json::from_str::<WsInMessage>(&text) {
                                    Ok(WsInMessage::UrlChange { url, title }) => {
                                        let config = reload_config(&app);
                                        let matched = find_matching_rule(&url, &config.rules);

                                        let update = if let Some(rule) = matched {
                                            let update = PageNotesUpdate {
                                                url: url.clone(),
                                                title: title.clone(),
                                                matched: true,
                                                rule_id: Some(rule.id.clone()),
                                                name: Some(rule.name.clone()),
                                                note: Some(rule.note.clone()),
                                            };
                                            // Send match to browser extension
                                            let out = WsOutMessage::NoteMatch {
                                                rule_id: rule.id.clone(),
                                                name: rule.name.clone(),
                                                note: rule.note.clone(),
                                            };
                                            let _ = write
                                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                                    serde_json::to_string(&out).unwrap().into(),
                                                ))
                                                .await;
                                            update
                                        } else {
                                            let update = PageNotesUpdate {
                                                url: url.clone(),
                                                title: title.clone(),
                                                matched: false,
                                                rule_id: None,
                                                name: None,
                                                note: None,
                                            };
                                            let _ = write
                                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                                    serde_json::to_string(&WsOutMessage::NoMatch).unwrap().into(),
                                                ))
                                                .await;
                                            update
                                        };

                                        // Update shared state
                                        *state.lock().await = Some(update.clone());

                                        // Emit event to Tauri frontend
                                        let _ = app.emit("page-notes-update", &update);
                                        // Also emit page-url-changed so PageNotesPanel receives the URL
                                        let _ = app.emit("page-url-changed", &serde_json::json!({ "url": update.url }));
                                    }
                                    Ok(WsInMessage::Ping) => {
                                        let _ = write
                                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                                serde_json::to_string(&WsOutMessage::Pong).unwrap().into(),
                                            ))
                                            .await;
                                    }
                                    Err(_) => {
                                        eprintln!("[page-notes] Failed to parse WS message: {}", text);
                                    }
                                }
                            }
                            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }
                }
                println!("[page-notes] Browser extension disconnected");
            });
        }
    });

    Ok(())
}
