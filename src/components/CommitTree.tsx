import { useMemo } from "react";
import type { GitLogEntry } from "../lib/types";

interface CommitTreeProps {
  entries: GitLogEntry[];
}

const LANE_COLORS = ["#60a5fa", "#c084fc", "#4ade80", "#facc15", "#f87171", "#fb923c", "#a78bfa"];

function laneColor(lane: number): string {
  return LANE_COLORS[lane % LANE_COLORS.length];
}

interface CommitNode {
  entry: GitLogEntry;
  lane: number;
  parentLanes: Map<string, number>; // parent hash → lane
}

/** Build a simple lane assignment from the commit list. */
function buildGraph(entries: GitLogEntry[]): CommitNode[] {
  const hashToIndex = new Map<string, number>();
  entries.forEach((e, i) => hashToIndex.set(e.fullHash, i));

  const hashToLane = new Map<string, number>();
  const nodes: CommitNode[] = [];

  for (let i = 0; i < entries.length; i++) {
    const entry = entries[i];

    // Assign lane: use parent's lane if this is a linear continuation, else new lane
    let lane: number;
    if (i === 0) {
      lane = 0;
    } else {
      const prevEntry = entries[i - 1];
      // If previous commit is our parent, stay on same lane
      if (entry.parents.includes(prevEntry.fullHash)) {
        lane = hashToLane.get(prevEntry.fullHash) ?? 0;
      } else {
        // Find an existing lane from any parent
        const parentLane = entry.parents
          .map((p) => hashToLane.get(p))
          .find((l) => l !== undefined);
        lane = parentLane ?? 0;
      }
    }

    hashToLane.set(entry.fullHash, lane);

    // Assign lanes to parents that don't have one yet
    const parentLanes = new Map<string, number>();
    for (const parent of entry.parents) {
      if (!hashToLane.has(parent)) {
        // New branch from this commit
        const newLane = nodes.length > 0 ? Math.max(...Array.from(hashToLane.values())) + 1 : 0;
        hashToLane.set(parent, newLane);
        parentLanes.set(parent, newLane);
      } else {
        parentLanes.set(parent, hashToLane.get(parent)!);
      }
    }

    nodes.push({ entry, lane, parentLanes });
  }

  return nodes;
}

export function CommitTree({ entries }: CommitTreeProps) {
  const nodes = useMemo(() => buildGraph(entries), [entries]);

  if (nodes.length === 0) {
    return <div className="diff-empty">暂无提交记录</div>;
  }

  // Determine total number of lanes
  const maxLane = nodes.reduce((max, n) => Math.max(max, n.lane), 0);

  return (
    <div className="commit-tree">
      {nodes.map((node, idx) => {
        const nextNode = nodes[idx + 1];
        return (
          <div key={node.entry.fullHash} className="commit-row">
            {/* Lane dots and lines */}
            <div className="commit-lanes" style={{ minWidth: `${(maxLane + 1) * 16}px` }}>
              {Array.from({ length: maxLane + 1 }, (_, laneIdx) => {
                const isCurrent = laneIdx === node.lane;
                const isParentLane = Array.from(node.parentLanes.values()).includes(laneIdx);
                const continuesToNext = nextNode && laneIdx === nextNode.lane;

                return (
                  <div key={laneIdx} className="commit-lane">
                    {/* Vertical line above */}
                    <div
                      className="commit-line"
                      style={{ background: (isCurrent || (idx > 0 && laneIdx === nodes[idx - 1]?.lane)) ? laneColor(laneIdx) : "transparent" }}
                    />
                    {/* Dot */}
                    {isCurrent && (
                      <div className="commit-dot" style={{ background: laneColor(laneIdx) }} />
                    )}
                    {/* Vertical line below */}
                    <div
                      className="commit-line"
                      style={{ background: (continuesToNext || isParentLane) ? laneColor(laneIdx) : "transparent" }}
                    />
                  </div>
                );
              })}
            </div>

            {/* Commit info */}
            <div className="commit-info">
              <span className="commit-msg">{node.entry.message}</span>
              <span className="commit-meta">
                <span className="log-hash">{node.entry.hash}</span>
                <span className="log-author">{node.entry.author}</span>
                <span className="log-date">{node.entry.date.split(" ")[0]}</span>
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
