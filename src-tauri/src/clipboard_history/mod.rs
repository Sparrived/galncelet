use std::collections::{VecDeque, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::Manager;

const MAX_ENTRIES: usize = 100;
const POLL_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: u64,
    pub text: String,
    pub timestamp: i64,
}

pub struct ClipboardHistoryState {
    entries: Arc<Mutex<VecDeque<ClipboardEntry>>>,
    next_id: Arc<Mutex<u64>>,
    last_hash: Arc<Mutex<u64>>,
    history_path: Option<PathBuf>,
}

impl ClipboardHistoryState {
    pub fn new(history_path: Option<PathBuf>) -> Self {
        let entries = history_path
            .as_ref()
            .and_then(|path| load_entries(path).ok())
            .unwrap_or_default();
        let next_id = entries.iter().map(|entry| entry.id).max().unwrap_or(0) + 1;
        let last_hash = entries
            .front()
            .map(|entry| text_hash(&entry.text))
            .unwrap_or_default();

        Self {
            entries: Arc::new(Mutex::new(entries)),
            next_id: Arc::new(Mutex::new(next_id)),
            last_hash: Arc::new(Mutex::new(last_hash)),
            history_path,
        }
    }

    fn add(&self, text: String) {
        let snapshot = {
            let mut entries = self.entries.lock().unwrap();
            let mut nid = self.next_id.lock().unwrap();
            entries.push_front(ClipboardEntry {
                id: *nid,
                text,
                timestamp: now_secs(),
            });
            *nid += 1;
            while entries.len() > MAX_ENTRIES {
                entries.pop_back();
            }
            entries.iter().cloned().collect::<Vec<_>>()
        };
        let _ = self.save_snapshot(&snapshot);
    }

    pub fn query(&self, q: Option<&str>) -> Vec<ClipboardEntry> {
        let entries = self.entries.lock().unwrap();
        match q {
            Some(s) if !s.is_empty() => {
                let lc = s.to_lowercase();
                entries.iter().filter(|e| e.text.to_lowercase().contains(&lc)).cloned().collect()
            }
            _ => entries.iter().cloned().collect(),
        }
    }

    pub fn remove(&self, id: u64) {
        let snapshot = {
            let mut entries = self.entries.lock().unwrap();
            entries.retain(|e| e.id != id);
            entries.iter().cloned().collect::<Vec<_>>()
        };
        let _ = self.save_snapshot(&snapshot);
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
        let _ = self.save_snapshot(&[]);
    }

    pub fn text_of(&self, id: u64) -> Option<String> {
        self.entries.lock().unwrap().iter().find(|e| e.id == id).map(|e| e.text.clone())
    }

    fn save_snapshot(&self, entries: &[ClipboardEntry]) -> Result<(), String> {
        let Some(path) = &self.history_path else {
            return Ok(());
        };
        let data = serde_json::to_string_pretty(entries)
            .map_err(|e| format!("Failed to serialize clipboard history: {e}"))?;
        fs::write(path, data).map_err(|e| format!("Failed to write clipboard history: {e}"))
    }
}

fn history_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(data_dir.join("clipboard-history.json"))
}

fn load_entries(path: &PathBuf) -> Result<VecDeque<ClipboardEntry>, String> {
    if !path.exists() {
        return Ok(VecDeque::new());
    }
    let data = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read clipboard history: {e}"))?;
    let mut entries: Vec<ClipboardEntry> = serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse clipboard history: {e}"))?;
    entries.truncate(MAX_ENTRIES);
    Ok(entries.into())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn text_hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub fn start_monitor(state: Arc<ClipboardHistoryState>) {
    std::thread::spawn(move || {
        let Ok(mut cb) = arboard::Clipboard::new() else { return };
        loop {
            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            let text = cb.get_text().unwrap_or_default();
            if text.is_empty() || text.len() > 10_000 { continue; }
            let hash = text_hash(&text);
            let mut last = state.last_hash.lock().unwrap();
            if hash == *last { continue; }
            *last = hash;
            drop(last);
            state.add(text);
        }
    });
}

#[tauri::command]
pub fn get_clipboard_history(state: tauri::State<'_, Arc<ClipboardHistoryState>>, query: Option<String>) -> Vec<ClipboardEntry> {
    state.query(query.as_deref())
}

#[tauri::command]
pub fn copy_to_clipboard(state: tauri::State<'_, Arc<ClipboardHistoryState>>, id: u64) -> Result<(), String> {
    let text = state.text_of(id).ok_or("not found")?;
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text).map_err(|e| e.to_string())?;
    *state.last_hash.lock().unwrap() = text_hash(&cb.get_text().unwrap_or_default());
    Ok(())
}

#[tauri::command]
pub fn delete_clipboard_entry(state: tauri::State<'_, Arc<ClipboardHistoryState>>, id: u64) {
    state.remove(id);
}

#[tauri::command]
pub fn clear_clipboard_history(state: tauri::State<'_, Arc<ClipboardHistoryState>>) {
    state.clear();
}

// ─── Plugin Setup ──────────────────────────────────────────────────

/// Initialize clipboard history plugin: create state, start monitor.
pub fn setup(app: &tauri::AppHandle) {
    let state = Arc::new(ClipboardHistoryState::new(history_path(app).ok()));
    app.manage(state.clone());
    start_monitor(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_history_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "galncelet-{name}-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }

    #[test]
    fn persists_entries_across_state_reloads() {
        let path = temp_history_path("clipboard-history");

        let state = ClipboardHistoryState::new(Some(path.clone()));
        state.add("first".to_string());
        state.add("second".to_string());

        let reloaded = ClipboardHistoryState::new(Some(path.clone()));
        let entries = reloaded.query(None);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "second");
        assert_eq!(entries[1].text, "first");
        assert_eq!(*reloaded.next_id.lock().unwrap(), 3);

        let _ = fs::remove_file(path);
    }
}
