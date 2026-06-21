use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileEntry {
    pub path: String,
    #[serde(rename = "statusCode")]
    pub status_code: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    #[serde(rename = "repoRoot")]
    pub repo_root: String,
    pub branch: String,
    #[serde(rename = "hasHead")]
    pub has_head: bool,
    pub files: Vec<GitFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub diff: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLogEntry {
    pub hash: String,
    pub full_hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub date: String,
    pub message: String,
}

fn run_git(repo_root: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(stderr);
    }
    Ok(stdout)
}

/// Run a git command and return stdout, stderr, and success status.
pub fn run_git_raw(repo_root: &str, args: &[&str]) -> (bool, String, String) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            (o.status.success(), stdout, stderr)
        }
        Err(e) => (false, String::new(), format!("Failed to run git: {e}")),
    }
}

/// Run an arbitrary git command string and return the result.
pub fn exec_git_command(repo_root: &str, command: &str) -> Result<(bool, String, String), String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(&parts)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((output.status.success(), stdout, stderr))
}

/// Resolve the repo root from a given path (or cwd if None).
pub fn resolve_repo_root(path: Option<&str>) -> Result<String, String> {
    let dir = path.unwrap_or(".");
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Not a git repository: {stderr}"));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(root)
}

/// Check if the repo has a HEAD commit.
fn has_head(repo_root: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "HEAD"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get current branch name. Returns "DETACHED" for detached HEAD.
fn get_branch(repo_root: &str) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["symbolic-ref", "--short", "HEAD"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => {
            // Fallback: try to describe HEAD
            let output = Command::new("git")
                .arg("-C")
                .arg(repo_root)
                .args(["describe", "--all", "--always", "HEAD"])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                }
                _ => "DETACHED".to_string(),
            }
        }
    }
}

/// Parse `git status --porcelain=v2` output (newline-separated) into file entries.
fn parse_status_porcelain(raw: &[u8]) -> Vec<GitFileEntry> {
    let text = String::from_utf8_lossy(raw);
    let mut files = Vec::new();

    for line_str in text.lines() {
        if line_str.is_empty() {
            continue;
        }

        // porcelain v2 format:
        // Ordinary: 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
        // Renamed:  2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path><tab><origPath>
        // Untracked: ? <path>
        // Ignored:   ! <path>
        if line_str.starts_with("? ") {
            let path = &line_str[2..];
            // Skip directory entries (trailing /)
            if path.ends_with('/') {
                continue;
            }
            files.push(GitFileEntry {
                path: path.to_string(),
                status_code: "?".to_string(),
                staged: false,
            });
            continue;
        }

        if line_str.starts_with("! ") {
            // Ignored - skip
            continue;
        }

        // For porcelain v2, lines start with 1 or 2 (regular/renamed/copied)
        if line_str.starts_with("1 ") || line_str.starts_with("2 ") {
            let parts: Vec<&str> = line_str.splitn(3, ' ').collect();
            if parts.len() < 3 {
                continue;
            }
            let rest = parts[2]; // everything after "1 " or "2 "

            // rest format: XY sub mH mI mW hH hI [Xscore origPath<tab>]path
            // XY is 2 chars at position 0-1
            if rest.len() < 2 {
                continue;
            }
            let xy = &rest[..2];
            let x = xy.chars().nth(0).unwrap_or('.');
            let y = xy.chars().nth(1).unwrap_or('.');

            // Parse remaining fields to find the path
            let fields: Vec<&str> = rest[3..].split(' ').collect(); // skip XY + space

            let (path, _status_code) = if line_str.starts_with("2 ") {
                // Renamed/copied - path is last field, may have tab
                if let Some(last) = fields.last() {
                    if let Some((p, _orig)) = last.split_once('\t') {
                        (p.to_string(), y.to_string())
                    } else {
                        (last.to_string(), y.to_string())
                    }
                } else {
                    continue;
                }
            } else {
                // Regular - path is last field
                if let Some(last) = fields.last() {
                    (last.to_string(), y.to_string())
                } else {
                    continue;
                }
            };

            // staged = index status is not '.'
            if x != '.' {
                files.push(GitFileEntry {
                    path: path.clone(),
                    status_code: x.to_string(),
                    staged: true,
                });
            }
            if y != '.' {
                files.push(GitFileEntry {
                    path,
                    status_code: y.to_string(),
                    staged: false,
                });
            }
        }
    }

    files
}

/// Get git status for a repo.
pub fn get_status(repo_path: Option<&str>) -> Result<GitStatus, String> {
    let repo_root = resolve_repo_root(repo_path)?;

    let status_raw = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["status", "--porcelain=v2", "--untracked-files=all"])
        .output()
        .map_err(|e| format!("Failed to run git status: {e}"))?;

    if !status_raw.status.success() {
        let stderr = String::from_utf8_lossy(&status_raw.stderr);
        return Err(format!("git status failed: {stderr}"));
    }

    let files = parse_status_porcelain(&status_raw.stdout);
    let branch = get_branch(&repo_root);
    let head = has_head(&repo_root);

    Ok(GitStatus {
        repo_root,
        branch,
        has_head: head,
        files,
    })
}

/// Stage a file (git add)
pub fn stage_file(repo_root: &str, file_path: &str) -> Result<(), String> {
    remove_stale_lock(repo_root);
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["add", "--", file_path])
        .output()
        .map_err(|e| format!("Failed to run git add: {e}"))?;
    if !status.status.success() {
        return Err(String::from_utf8_lossy(&status.stderr).to_string());
    }
    Ok(())
}

/// Unstage a file (git reset HEAD)
pub fn unstage_file(repo_root: &str, file_path: &str) -> Result<(), String> {
    remove_stale_lock(repo_root);
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["reset", "HEAD", "--", file_path])
        .output()
        .map_err(|e| format!("Failed to run git reset: {e}"))?;
    if !status.status.success() {
        return Err(String::from_utf8_lossy(&status.stderr).to_string());
    }
    Ok(())
}

/// Untrack a file (git rm --cached) — removes from index without deleting the file.
pub fn untrack_file(repo_root: &str, file_path: &str) -> Result<(), String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rm", "--cached", "--", file_path])
        .output()
        .map_err(|e| format!("Failed to run git rm --cached: {e}"))?;
    if !status.status.success() {
        return Err(String::from_utf8_lossy(&status.stderr).to_string());
    }
    Ok(())
}

/// Discard changes to a file (git checkout -- / git clean)
pub fn discard_file(repo_root: &str, file_path: &str, status_code: &str) -> Result<(), String> {
    if status_code == "?" {
        // Untracked: remove file
        let _ = std::fs::remove_file(format!("{repo_root}/{file_path}"));
    } else {
        // Tracked: checkout from HEAD
        let status = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(["checkout", "HEAD", "--", file_path])
            .output()
            .map_err(|e| format!("Failed to run git checkout: {e}"))?;
        if !status.status.success() {
            return Err(String::from_utf8_lossy(&status.stderr).to_string());
        }
    }
    Ok(())
}

/// Commit staged changes
pub fn commit(repo_root: &str, message: &str) -> Result<String, String> {
    remove_stale_lock(repo_root);
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["commit", "-m", message])
        .output()
        .map_err(|e| format!("Failed to run git commit: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Pull from remote
pub fn pull(repo_root: &str) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["pull", "--rebase"])
        .output()
        .map_err(|e| format!("Failed to run git pull: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Push to remote
pub fn push(repo_root: &str) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["push"])
        .output()
        .map_err(|e| format!("Failed to run git push: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// List all branches (local + remote) with current/upstream info.
pub fn list_branches(repo_root: &str) -> Result<Vec<GitBranch>, String> {
    // Use for-each-ref for reliable parsing
    let format = "%(refname:short)\t%(if)%(HEAD)%(then)1%(else)0%(end)\t%(upstream:short)\t%(refname)";
    let output = run_git(repo_root, &["for-each-ref", &format!("--format={format}"), "refs/heads", "refs/remotes"])?;

    let mut branches = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[0].to_string();
        let is_current = parts[1] == "1";
        let upstream = if parts[2].is_empty() { None } else { Some(parts[2].to_string()) };
        let is_remote = parts[3].starts_with("refs/remotes/");

        // Skip HEAD symbolic refs like "origin/HEAD"
        if name.ends_with("/HEAD") {
            continue;
        }

        branches.push(GitBranch {
            name,
            is_current,
            is_remote,
            upstream,
        });
    }

    // Sort: local branches first (current on top), then remote branches
    branches.sort_by(|a, b| {
        match (a.is_remote, b.is_remote) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => {
                if a.is_current {
                    std::cmp::Ordering::Less
                } else if b.is_current {
                    std::cmp::Ordering::Greater
                } else {
                    a.name.cmp(&b.name)
                }
            }
        }
    });

    Ok(branches)
}

/// Switch to a different branch.
pub fn checkout_branch(repo_root: &str, branch: &str) -> Result<String, String> {
    let output = run_git(repo_root, &["checkout", branch])?;
    Ok(output.trim().to_string())
}

/// Fetch from all remotes with prune.
pub fn git_fetch(repo_root: &str) -> Result<String, String> {
    let output = run_git(repo_root, &["fetch", "--all", "--prune"])?;
    Ok(output.trim().to_string())
}

/// Get recent commit log.
/// Uses raw bytes to avoid NUL-byte corruption from from_utf8_lossy.
pub fn git_log(repo_root: &str, max_count: usize) -> Result<Vec<GitLogEntry>, String> {
    let format_arg = format!("-{}", max_count);
    let format_str = "%h%x00%H%x00%P%x00%an%x00%ai%x00%s";
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["log", &format_arg, &format!("--format={format_str}"), "--no-color"])
        .output()
        .map_err(|e| format!("Failed to run git log: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Empty repo or other error — return empty list for "no commits" case
        if stderr.contains("does not have any commits") {
            return Ok(Vec::new());
        }
        return Err(stderr.to_string());
    }

    let raw = &output.stdout;
    let mut entries = Vec::new();

    // Split on newlines (each commit is one line), then split fields on NUL bytes
    for line in raw.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line
            .split(|&b| b == 0)
            .map(|s| std::str::from_utf8(s).unwrap_or(""))
            .collect();
        if parts.len() < 6 {
            continue;
        }
        let parents: Vec<String> = parts[2]
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        entries.push(GitLogEntry {
            hash: parts[0].to_string(),
            full_hash: parts[1].to_string(),
            parents,
            author: parts[3].to_string(),
            date: parts[4].to_string(),
            message: parts[5].to_string(),
        });
    }

    Ok(entries)
}

/// Get unified diff for a specific file.
pub fn get_file_diff(
    repo_root: &str,
    file_path: &str,
    staged: bool,
) -> Result<GitDiff, String> {
    let mut args = vec!["diff", "--no-ext-diff"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(file_path);

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run git diff: {e}"))?;

    // For untracked files, `git diff` returns empty. Show file content as a new-file diff.
    let diff_text = if output.stdout.is_empty() && !staged {
        // Check if the file is untracked by trying to show its content
        let file_content = std::fs::read_to_string(format!("{repo_root}/{file_path}"));
        match file_content {
            Ok(content) => {
                let line_count = content.lines().count();
                let mut result = format!(
                    "diff --git a/{file_path} b/{file_path}\n\
                     new file mode 100644\n\
                     --- /dev/null\n\
                     +++ b/{file_path}\n\
                     @@ -0,0 +1,{line_count} @@\n"
                );
                for line in content.lines() {
                    result.push_str(&format!("+{line}\n"));
                }
                result
            }
            Err(_) => String::new(),
        }
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    Ok(GitDiff {
        file_path: file_path.to_string(),
        diff: diff_text,
        staged,
    })
}

/// Information about a submodule or nested repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoduleInfo {
    pub path: String,
    pub name: String,
}

/// Remove stale git index.lock files if they exist and are older than 1 second.
/// Recursively checks .git directory and all nested submodules.
fn remove_stale_lock(repo_root: &str) {
    let git_dir = std::path::Path::new(repo_root).join(".git");
    remove_stale_lock_in_dir(&git_dir);
}

fn remove_stale_lock_in_dir(dir: &std::path::Path) {
    // Remove index.lock in this directory
    let lock_path = dir.join("index.lock");
    if lock_path.exists() {
        if let Ok(metadata) = std::fs::metadata(&lock_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    // Remove if older than 1 second
                    if elapsed.as_millis() > 1000 {
                        let _ = std::fs::remove_file(&lock_path);
                    }
                }
            }
        }
    }

    // Recursively check all subdirectories for lock files
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remove_stale_lock_in_dir(&path);
            }
        }
    }
}

/// Stage all changes (tracked + untracked).
pub fn stage_all(repo_root: &str) -> Result<(), String> {
    // Remove stale lock file if exists
    remove_stale_lock(repo_root);

    // Stage tracked files
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["add", "-u"])
        .output()
        .map_err(|e| format!("Failed to stage tracked files: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git add -u failed: {}", stderr));
    }

    // Stage untracked files
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["add", "--", "."])
        .output()
        .map_err(|e| format!("Failed to stage untracked files: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git add . failed: {}", stderr));
    }

    Ok(())
}

/// Get a summary of all staged changes (combined diff) for AI commit generation.
pub fn get_staged_diff_summary(repo_root: &str) -> Result<String, String> {
    // Get staged diff (all files)
    let diff_output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["diff", "--cached", "--stat"])
        .output()
        .map_err(|e| format!("Failed to get staged stat: {e}"))?;

    let stat = String::from_utf8_lossy(&diff_output.stdout).to_string();

    // Get actual diff (limited to avoid huge output)
    let diff_detail = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["diff", "--cached", "--no-color"])
        .output()
        .map_err(|e| format!("Failed to get staged diff: {e}"))?;

    let detail = String::from_utf8_lossy(&diff_detail.stdout).to_string();

    Ok(format!("{}\n{}", stat, detail))
}

/// List git submodules in a repository.
pub fn list_submodules(repo_root: &str) -> Vec<SubmoduleInfo> {
    let output = match Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["submodule", "status"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut submodules = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: [<prefix>]<hash> <path> [(<branch>)]
        // prefix can be '-', '+', ' ', 'U'
        // Skip the prefix char and hash, extract path
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let path = parts[1].to_string();
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            submodules.push(SubmoduleInfo { path, name });
        }
    }

    submodules
}
