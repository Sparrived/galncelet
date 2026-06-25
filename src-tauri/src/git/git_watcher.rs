use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Manages file system watchers for git repositories.
/// Watches .git directory and working tree for changes, emitting events to the frontend.
pub struct GitWatcherManager {
    watchers: Arc<Mutex<Vec<WatcherEntry>>>,
    app_handle: AppHandle,
}

struct WatcherEntry {
    repo_path: String,
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl GitWatcherManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            watchers: Arc::new(Mutex::new(Vec::new())),
            app_handle,
        }
    }

    /// Start watching a git repository for changes.
    /// If already watching this repo, does nothing.
    pub fn watch(&self, repo_path: &str) -> Result<(), String> {
        let mut watchers = self.watchers.lock().unwrap();

        // Check if already watching
        if watchers.iter().any(|w| w.repo_path == repo_path) {
            return Ok(());
        }

        let repo = PathBuf::from(repo_path);
        let git_dir = repo.join(".git");

        if !git_dir.exists() {
            return Err(format!("Not a git repository: {}", repo_path));
        }

        let app_handle = self.app_handle.clone();
        let repo_root = repo_path.to_string();

        // Create debounced watcher (500ms debounce)
        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            move |events: Result<Vec<DebouncedEvent>, notify::Error>| {
                if let Ok(events) = events {
                    // Filter out .git internal files we don't care about
                    let has_relevant_change = events.iter().any(|e| {
                        let path = &e.path;
                        // Skip .git internal lock/config files
                        let path_str = path.to_string_lossy();
                        if path_str.contains(".git") {
                            // Only care about HEAD, index, refs changes
                            let relative = path.strip_prefix(&repo_root).unwrap_or(path);
                            let rel_str = relative.to_string_lossy();
                            return rel_str == ".git\\HEAD"
                                || rel_str == ".git/HEAD"
                                || rel_str.contains("refs/")
                                || rel_str == ".git\\index"
                                || rel_str == ".git/index";
                        }
                        true // Working tree changes are always relevant
                    });

                    if has_relevant_change {
                        let _ = app_handle.emit("git-changed", &repo_root);
                    }
                }
            },
        )
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        // Watch the repository root recursively
        debouncer
            .watcher()
            .watch(&repo, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;

        watchers.push(WatcherEntry {
            repo_path: repo_path.to_string(),
            _debouncer: debouncer,
        });

        Ok(())
    }

    /// Stop watching a git repository.
    pub fn unwatch(&self, repo_path: &str) {
        let mut watchers = self.watchers.lock().unwrap();
        watchers.retain(|w| w.repo_path != repo_path);
    }

    /// Stop watching all repositories.
    pub fn unwatch_all(&self) {
        let mut watchers = self.watchers.lock().unwrap();
        watchers.clear();
    }
}
