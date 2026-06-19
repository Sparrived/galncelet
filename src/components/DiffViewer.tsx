import { useMemo } from "react";
import type { GitDiff } from "../lib/types";

interface DiffLine {
  type: "header" | "hunk" | "add" | "del" | "ctx";
  text: string;
}

function parseDiff(raw: string): DiffLine[] {
  if (!raw) return [];
  const lines = raw.split("\n");
  return lines.map((line) => {
    if (line.startsWith("@@")) return { type: "hunk", text: line };
    if (line.startsWith("+++") || line.startsWith("---") || line.startsWith("diff "))
      return { type: "header", text: line };
    if (line.startsWith("+")) return { type: "add", text: line };
    if (line.startsWith("-")) return { type: "del", text: line };
    return { type: "ctx", text: line };
  });
}

interface DiffViewerProps {
  diff: GitDiff | null;
  statusCode?: string;
}

export function DiffViewer({ diff, statusCode }: DiffViewerProps) {
  const parsed = useMemo(() => (diff ? parseDiff(diff.diff) : []), [diff]);

  if (!diff) {
    return (
      <div className="diff-empty">
        <span>选择文件以查看差异</span>
      </div>
    );
  }

  if (parsed.length === 0) {
    const msg = statusCode === "?" ? "新文件 — 暂存后可查看完整内容" : "该文件无可用差异";
    return (
      <div className="diff-empty">
        <span>{msg}</span>
      </div>
    );
  }

  return (
    <div className="diff-scroll">
      <pre className="diff-pre">
        {parsed.map((line, i) => (
          <div key={i} className={`diff-line diff-${line.type}`}>
            <span className="diff-text">{line.text || " "}</span>
          </div>
        ))}
      </pre>
    </div>
  );
}
