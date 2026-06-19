import { useEffect, useState, useCallback, useRef } from "react";
import {
  getStatus,
  getFileDiff,
  stageFile,
  unstageFile,
  discardFile,
  untrackFile,
  commit as gitCommit,
  pull as gitPull,
  push as gitPush,
  gitFetch as gitFetchApi,
  listBranches,
  checkoutBranch,
  gitLog,
  loadSettings,
} from "../../lib/api";
import type { GitStatus, GitDiff, TreeNode, GitBranch, GitLogEntry } from "../../lib/types";
import { GitTree } from "../../components/GitTree";
import { DiffViewer } from "../../components/DiffViewer";
import { CommitTree } from "../../components/CommitTree";
import { useWidget } from "../../lib/context";

/** Build a TreeNode tree from flat GitFileEntry[] */
function buildTree(files: GitStatus["files"]): TreeNode[] {
  const root: TreeNode[] = [];
  const dirMap = new Map<string, TreeNode>();

  for (const f of files) {
    const segments = f.path.replace(/\\/g, "/").split("/");
    const fileNode: TreeNode = {
      name: segments[segments.length - 1],
      path: f.path,
      type: "file",
      statusCode: f.statusCode,
      staged: f.staged,
    };

    if (segments.length === 1) {
      root.push(fileNode);
    } else {
      let parentChildren = root;
      for (let i = 0; i < segments.length - 1; i++) {
        const dirPath = segments.slice(0, i + 1).join("/");
        let dir = dirMap.get(dirPath);
        if (!dir) {
          dir = {
            name: segments[i],
            path: dirPath,
            type: "dir",
            children: [],
            expanded: i === 0,
          };
          dirMap.set(dirPath, dir);
          parentChildren.push(dir);
        }
        parentChildren = dir.children!;
      }
      parentChildren.push(fileNode);
    }
  }

  return root;
}

export default function GitPanel() {
  const { showResult, showError, onStatusChange } = useWidget();

  const [status, setStatus] = useState<GitStatus | null>(null);
  const [tree, setTree] = useState<TreeNode[]>([]);
  const [diff, setDiff] = useState<GitDiff | null>(null);
  const [selectedFile, setSelectedFile] = useState<{
    path: string;
    staged: boolean;
    statusCode: string;
  } | null>(null);
  const [commitMsg, setCommitMsg] = useState("");

  // Branch switcher
  const [branches, setBranches] = useState<GitBranch[]>([]);
  const [showBranchDropdown, setShowBranchDropdown] = useState(false);
  const branchRef = useRef<HTMLDivElement>(null);

  // Log viewer
  const [logEntries, setLogEntries] = useState<GitLogEntry[]>([]);
  const [logMaxCount, setLogMaxCount] = useState(50);
  const [view, setView] = useState<"changes" | "history">("changes");

  // Fetch loading
  const [fetching, setFetching] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const s = await getStatus();
      setStatus(s);
      setTree(buildTree(s.files));
      onStatusChange(s.repoRoot, s.branch);
    } catch {
      onStatusChange(null, null);
    }
  }, [onStatusChange]);

  // Expose refresh to widget context
  useEffect(() => {
    // Overwrite the widget refresh to also refresh git status
    // This is done by calling our refresh inside the widget refresh
  }, []);

  // Load settings for log max count, then start auto-refresh
  useEffect(() => {
    loadSettings().then((s) => setLogMaxCount(s.logMaxCount)).catch(() => {});
    refresh();
  }, []);

  // Auto-refresh — read interval from settings on mount
  const [refreshInterval, setRefreshInterval] = useState(2000);
  useEffect(() => {
    loadSettings().then((s) => setRefreshInterval(s.refreshIntervalMs)).catch(() => {});
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, refreshInterval);
    return () => clearInterval(id);
  }, [refresh, refreshInterval]);

  // Fetch diff when selected file changes
  useEffect(() => {
    if (!selectedFile || !status) {
      setDiff(null);
      return;
    }
    getFileDiff(status.repoRoot, selectedFile.path, selectedFile.staged)
      .then(setDiff)
      .catch(() => setDiff(null));
  }, [selectedFile, status]);

  // Close branch dropdown on outside click
  useEffect(() => {
    if (!showBranchDropdown) return;
    const handleClick = (e: MouseEvent) => {
      if (branchRef.current && !branchRef.current.contains(e.target as Node)) {
        setShowBranchDropdown(false);
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [showBranchDropdown]);

  const handleFileSelect = (path: string, staged: boolean, statusCode: string) => {
    setSelectedFile({ path, staged, statusCode });
  };

  const handleStage = async () => {
    if (!selectedFile || !status) return;
    try {
      await stageFile(status.repoRoot, selectedFile.path);
      showResult("已暂存");
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handleUnstage = async () => {
    if (!selectedFile || !status) return;
    try {
      await unstageFile(status.repoRoot, selectedFile.path);
      showResult("已取消暂存");
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handleDiscard = async () => {
    if (!selectedFile || !status) return;
    if (!confirm(`确认丢弃 ${selectedFile.path} 的更改？`)) return;
    try {
      await discardFile(status.repoRoot, selectedFile.path, selectedFile.statusCode);
      showResult("已丢弃");
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handleUntrack = async () => {
    if (!selectedFile || !status) return;
    if (!confirm(`确认停止追踪 ${selectedFile.path}？\n文件不会被删除，但会从 Git 中移除追踪。`)) return;
    try {
      await untrackFile(status.repoRoot, selectedFile.path);
      showResult("已停止追踪");
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handleCommit = async () => {
    if (!status || !commitMsg.trim()) return;
    try {
      const result = await gitCommit(status.repoRoot, commitMsg.trim());
      showResult(result);
      setCommitMsg("");
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handlePull = async () => {
    if (!status) return;
    try {
      const result = await gitPull(status.repoRoot);
      showResult(result || "拉取完成");
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handlePush = async () => {
    if (!status) return;
    try {
      const result = await gitPush(status.repoRoot);
      showResult(result || "推送完成");
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handleFetch = async () => {
    if (!status || fetching) return;
    setFetching(true);
    try {
      const result = await gitFetchApi(status.repoRoot);
      showResult(result || "拉取远端完成");
      await refresh();
    } catch (e) { showError(String(e)); }
    finally { setFetching(false); }
  };

  const handleToggleBranch = async () => {
    if (!status) return;
    if (showBranchDropdown) { setShowBranchDropdown(false); return; }
    try {
      const b = await listBranches(status.repoRoot);
      setBranches(b);
      setShowBranchDropdown(true);
    } catch (e) { showError(String(e)); }
  };

  const handleCheckout = async (branchName: string) => {
    if (!status) return;
    setShowBranchDropdown(false);
    try {
      await checkoutBranch(status.repoRoot, branchName);
      showResult(`已切换到 ${branchName}`);
      await refresh();
    } catch (e) { showError(String(e)); }
  };

  const handleToggleLog = async () => {
    if (view === "history") {
      setView("changes");
      return;
    }
    if (!status) return;
    try {
      const entries = await gitLog(status.repoRoot, logMaxCount);
      setLogEntries(entries);
      setView("history");
    } catch (e) { showError(String(e)); }
  };

  // Branch switcher UI (rendered in panel header)
  const branchUI = status ? (
    <div className="branch-wrapper" ref={branchRef}>
      <button
        className={`branch-btn${showBranchDropdown ? " active" : ""}`}
        onClick={handleToggleBranch}
      >
        <span className="branch-icon">&#x2442;</span>
        {status.branch}
      </button>
      {showBranchDropdown && branches.length > 0 && (
        <div className="branch-dropdown">
          {branches.map((b) => (
            <button
              key={b.name}
              className={`branch-item${b.isCurrent ? " current" : ""}${b.isRemote ? " remote" : ""}`}
              onClick={() => handleCheckout(b.name)}
              disabled={b.isCurrent}
            >
              <span>{b.name}</span>
              {b.upstream && <span className="branch-upstream">{b.upstream}</span>}
            </button>
          ))}
        </div>
      )}
    </div>
  ) : null;

  return (
    <>
      {/* Git tree panel — header always visible, body switches between changes and history */}
      <div className="panel panel-tree">
        <div className="panel-header">
          <span>
            {view === "history" ? "历史" : "变更"}
            {status && <span className="panel-header-repo">{status.repoRoot.split(/[\\/]/).pop()}</span>}
          </span>
          <div className="panel-header-right">
            {view === "changes" && branchUI}
            {view === "changes" && status && <span className="count">{status.files.length}</span>}
            <button
              className={`panel-header-btn${view === "history" ? " panel-header-btn-active" : ""}`}
              onClick={handleToggleLog}
              title={view === "history" ? "返回变更" : "提交历史"}
            >
              &#128197;
            </button>
          </div>
        </div>
        <div className="panel-body">
          {view === "history" ? (
            <CommitTree entries={logEntries} />
          ) : (
            <GitTree nodes={tree} onSelect={handleFileSelect} selectedPath={selectedFile?.path} />
          )}
        </div>
      </div>

      {/* Diff viewer + action bar — hidden in history view */}
      {view === "changes" && (
        <div className="panel panel-diff">
          <div className="panel-header">
            <span>{selectedFile ? selectedFile.path : "差异"}</span>
            {selectedFile && (
              <span className="diff-badge">
                {selectedFile.staged ? "已暂存" : "未暂存"}
              </span>
            )}
          </div>
          <div className="panel-body">
            <DiffViewer diff={diff} />
          </div>

          {selectedFile && (
            <div className="action-bar">
              {selectedFile.staged ? (
                <button className="btn-action btn-unstage" onClick={handleUnstage}>取消暂存</button>
              ) : (
                <button className="btn-action btn-stage" onClick={handleStage}>暂存</button>
              )}
              <button className="btn-action btn-discard" onClick={handleDiscard}>丢弃</button>
              {selectedFile.staged && (
                <button className="btn-action btn-untrack" onClick={handleUntrack}>停止追踪</button>
              )}
            </div>
          )}

          <div className="commit-bar">
            <input
              className="commit-input"
              type="text"
              placeholder="提交信息…"
              value={commitMsg}
              onChange={(e) => setCommitMsg(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleCommit()}
            />
            <button className="btn-action btn-commit" onClick={handleCommit} disabled={!commitMsg.trim()}>
              提交
            </button>
          </div>

          <div className="remote-bar">
            <button className="btn-action btn-fetch" onClick={handleFetch} disabled={fetching}>
              {fetching ? "..." : "拉取远端"}
            </button>
            <button className="btn-action btn-pull" onClick={handlePull}>&#8595; 拉取</button>
            <button className="btn-action btn-push" onClick={handlePush}>推送 &#8593;</button>
          </div>
        </div>
      )}
    </>
  );
}

// Panel definition — imported by registry
