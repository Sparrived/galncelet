use std::collections::{VecDeque, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;

const MAX_ENTRIES: usize = 100;
const POLL_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, Serialize)]
pub struct ClipboardEntry {
    pub id: u64,
    pub text: String,
    pub timestamp: i64,
}

pub struct ClipboardHistoryState {
    entries: Arc<Mutex<VecDeque<ClipboardEntry>>>,
    next_id: Arc<Mutex<u64>>,
    last_hash: Arc<Mutex<u64>>,
}

impl ClipboardHistoryState {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::new())),
            next_id: Arc::new(Mutex::new(1)),
            last_hash: Arc::new(Mutex::new(0)),
        }
    }

    fn add(&self, text: String) {
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
        self.entries.lock().unwrap().retain(|e| e.id != id);
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    pub fn text_of(&self, id: u64) -> Option<String> {
        self.entries.lock().unwrap().iter().find(|e| e.id == id).map(|e| e.text.clone())
    }
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
