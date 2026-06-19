import { useState } from "react";
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

function statusCodeClass(code: string): string {
  if (code === "?") return "status-untracked";
  if (code === "D") return "status-deleted";
  if (code === "A") return "status-added";
  return "status-modified";
}

interface GitTreeProps {
  nodes: TreeNode[];
  onSelect: (path: string, staged: boolean, statusCode: string) => void;
  selectedPath?: string;
  depth?: number;
}

export function GitTree({ nodes, onSelect, selectedPath, depth = 0 }: GitTreeProps) {
  // Sort: dirs first, then files; both alphabetical
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
        />
      ))}
    </ul>
  );
}

interface TreeItemProps {
  node: TreeNode;
  onSelect: (path: string, staged: boolean, statusCode: string) => void;
  selectedPath?: string;
  depth: number;
}

function TreeItem({ node, onSelect, selectedPath, depth }: TreeItemProps) {
  const [expanded, setExpanded] = useState(node.expanded ?? true);

  if (node.type === "dir") {
    return (
      <li className="tree-dir">
        <div
          className="tree-row"
          onClick={() => setExpanded((e) => !e)}
        >
          <span className="tree-chevron">{expanded ? "▼" : "▶"}</span>
          <span className="tree-dirname">{node.name}</span>
        </div>
        {expanded && node.children && (
          <GitTree
            nodes={node.children}
            onSelect={onSelect}
            selectedPath={selectedPath}
            depth={depth + 1}
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
        <span className={`status-badge ${statusCodeClass(code)}`}>
          {STATUS_LABELS[code] ?? code}
        </span>
        <span className="tree-filename">{node.name}</span>
        {node.staged && <span className="staged-dot" title="已暂存">●</span>}
      </div>
    </li>
  );
}
