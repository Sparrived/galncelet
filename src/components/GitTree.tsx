import React, { useState } from "react";
import type { TreeNode } from "../lib/types";

const STATUS_LABELS: Record<string, string> = {
  M: "改",
  A: "增",
  D: "删",
  R: "移",
  C: "复",
  "?": "新",
  "!": "忽",
  U: "冲",
};

function statusCodeClass(code: string, staged: boolean): string {
  const prefix = staged ? "staged-" : "";
  if (code === "?") return `${prefix}status-untracked`;
  if (code === "D") return `${prefix}status-deleted`;
  if (code === "A") return `${prefix}status-added`;
  return `${prefix}status-modified`;
}

interface GitTreeActions {
  onStage?: () => void;
  onUnstage?: () => void;
  onDiscard?: () => void;
  onUntrack?: () => void;
  /** Whether the selected file is staged */
  selectedStaged?: boolean;
}

interface GitTreeProps {
  nodes: TreeNode[];
  onSelect: (path: string, staged: boolean, statusCode: string) => void;
  selectedPath?: string;
  depth?: number;
  actions?: GitTreeActions;
}

export const GitTree = React.memo(function GitTree({ nodes, onSelect, selectedPath, depth = 0, actions }: GitTreeProps) {
  const sorted = [...nodes].sort((a, b) => {
    if (a.type !== b.type) return a.type === "dir" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <ul className="tree-list" style={{ paddingLeft: depth > 0 ? 16 : 0 }}>
      {sorted.map((node) => (
        <TreeItem
          key={node.path}
          node={node}
          onSelect={onSelect}
          selectedPath={selectedPath}
          depth={depth}
          actions={actions}
        />
      ))}
    </ul>
  );
});

interface TreeItemProps {
  node: TreeNode;
  onSelect: (path: string, staged: boolean, statusCode: string) => void;
  selectedPath?: string;
  depth: number;
  actions?: GitTreeActions;
}

function TreeItem({ node, onSelect, selectedPath, depth, actions }: TreeItemProps) {
  const [expanded, setExpanded] = useState(node.expanded ?? true);

  if (node.type === "dir") {
    return (
      <li className={`tree-dir${node.isSubmodule ? " tree-dir-submodule" : ""}`}>
        <div className="tree-row" onClick={() => setExpanded((e) => !e)}>
          <span className="tree-chevron">{expanded ? "▼" : "▶"}</span>
          <span className="tree-dirname">{node.name}</span>
        </div>
        {expanded && node.children && (
          <GitTree
            nodes={node.children}
            onSelect={onSelect}
            selectedPath={selectedPath}
            depth={depth + 1}
            actions={actions}
          />
        )}
      </li>
    );
  }

  const isSelected = node.path === selectedPath;
  const code = node.statusCode ?? "?";

  return (
    <li className={`tree-file${isSelected ? " selected" : ""}`}>
      <div
        className="tree-row"
        onClick={() => onSelect(node.path, node.staged ?? false, node.statusCode ?? "?")}
      >
        <span className={`status-badge ${statusCodeClass(code, node.staged ?? false)}`}>
          {STATUS_LABELS[code] ?? code}
        </span>
        <span className="tree-filename">{node.name}</span>
        {isSelected && actions ? (
          <span className="tree-actions">
            {actions.selectedStaged ? (
              <button className="tree-action-btn tree-action-unstage" onClick={(e) => { e.stopPropagation(); actions.onUnstage?.(); }} title="取消暂存">▼</button>
            ) : (
              <button className="tree-action-btn tree-action-stage" onClick={(e) => { e.stopPropagation(); actions.onStage?.(); }} title="暂存">▲</button>
            )}
            <button className="tree-action-btn tree-action-discard" onClick={(e) => { e.stopPropagation(); actions.onDiscard?.(); }} title="丢弃">✕</button>
            {actions.selectedStaged && (
              <button className="tree-action-btn tree-action-untrack" onClick={(e) => { e.stopPropagation(); actions.onUntrack?.(); }} title="停止追踪">⊘</button>
            )}
          </span>
        ) : (
          node.staged && <span className="staged-dot" title="已暂存">●</span>
        )}
      </div>
    </li>
  );
}
