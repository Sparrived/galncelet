import { useState, useRef, useEffect, useCallback } from "react";
import { execGitCommand, type GitCommandResult } from "../lib/api";

interface ConsoleEntry {
  id: number;
  type: "command" | "output" | "error";
  text: string;
}

interface GitConsoleProps {
  repoRoot: string;
  onClose: () => void;
  externalLog?: Array<{ type: "command" | "output" | "error"; text: string }>;
  onLog?: (type: "command" | "output" | "error", text: string) => void;
}

export function GitConsole({ repoRoot, onClose, externalLog = [], onLog }: GitConsoleProps) {
  const [entries, setEntries] = useState<ConsoleEntry[]>([]);
  const [input, setInput] = useState("");
  const [isExecuting, setIsExecuting] = useState(false);
  const outputRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const nextId = useRef(1);
  const lastExternalLogLength = useRef(0);

  // Sync external log entries
  useEffect(() => {
    if (externalLog.length > lastExternalLogLength.current) {
      const newEntries = externalLog.slice(lastExternalLogLength.current).map((e) => ({
        id: nextId.current++,
        type: e.type,
        text: e.text,
      }));
      setEntries((prev) => [...prev, ...newEntries]);
      lastExternalLogLength.current = externalLog.length;
    }
  }, [externalLog]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [entries]);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const addEntry = useCallback((type: ConsoleEntry["type"], text: string) => {
    const entry: ConsoleEntry = {
      id: nextId.current++,
      type,
      text,
    };
    setEntries((prev) => [...prev, entry]);
    // Also log to parent if callback provided
    if (onLog) {
      onLog(type, text);
    }
  }, [onLog]);

  const handleExecute = useCallback(async () => {
    const cmd = input.trim();
    if (!cmd || isExecuting) return;

    addEntry("command", `$ git ${cmd}`);
    setInput("");
    setIsExecuting(true);

    try {
      const result: GitCommandResult = await execGitCommand(repoRoot, cmd);
      if (result.stdout) {
        addEntry("output", result.stdout.trimEnd());
      }
      if (result.stderr) {
        addEntry("error", result.stderr.trimEnd());
      }
      if (!result.stdout && !result.stderr) {
        addEntry("output", "(no output)");
      }
    } catch (e) {
      addEntry("error", String(e));
    } finally {
      setIsExecuting(false);
    }
  }, [input, repoRoot, isExecuting, addEntry]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleExecute();
    } else if (e.key === "l" && e.ctrlKey) {
      e.preventDefault();
      setEntries([]);
    }
  }, [handleExecute]);

  const handleClear = useCallback(() => {
    setEntries([]);
    lastExternalLogLength.current = 0;
  }, []);

  return (
    <div className="git-console-overlay">
      <div className="git-console">
        <div className="git-console-header">
          <span className="git-console-title">Git Console</span>
          <span className="git-console-repo">{repoRoot.split(/[\\/]/).pop()}</span>
          <button className="git-console-clear" onClick={handleClear} title="清屏">Clear</button>
          <button className="git-console-close" onClick={onClose}>✕</button>
        </div>
        <div className="git-console-output" ref={outputRef}>
          {entries.length === 0 && (
            <div className="git-console-welcome">
              输入 git 命令执行，例如: status, add -A, commit -m "msg"
            </div>
          )}
          {entries.map((entry) => (
            <div key={entry.id} className={`git-console-entry git-console-${entry.type}`}>
              <pre>{entry.text}</pre>
            </div>
          ))}
        </div>
        <div className="git-console-input-row">
          <span className="git-console-prompt">$</span>
          <input
            ref={inputRef}
            className="git-console-input"
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="输入 git 命令..."
            disabled={isExecuting}
          />
        </div>
      </div>
    </div>
  );
}
