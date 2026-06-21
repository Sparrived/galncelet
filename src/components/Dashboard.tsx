import { useEffect, useRef, useState } from "react";
import type { AmkrMetrics, UsageStats } from "../lib/types";

interface DashboardProps {
  metrics: AmkrMetrics | null;
  loading: boolean;
}

/** 数字变化时带闪烁高亮效果的组件 */
function AnimatedNumber({ value, className }: { value: string; className?: string }) {
  const [flash, setFlash] = useState(false);
  const prevRef = useRef(value);

  useEffect(() => {
    if (prevRef.current !== value) {
      prevRef.current = value;
      setFlash(true);
      const timer = setTimeout(() => setFlash(false), 400);
      return () => clearTimeout(timer);
    }
  }, [value]);

  return <span className={`${className ?? ""} dash-animated-num${flash ? " dash-num-flash" : ""}`}>{value}</span>;
}

function fmt(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(1) + "K";
  return n.toLocaleString();
}

function fmtMs(ms: number): string {
  if (ms >= 1000) return (ms / 1000).toFixed(1) + "s";
  return Math.round(ms) + "ms";
}

function fmtUptime(startedAt: string): string {
  const diff = Date.now() - new Date(startedAt).getTime();
  const h = Math.floor(diff / 3_600_000);
  const m = Math.floor((diff % 3_600_000) / 60_000);
  if (h > 24) return `${Math.floor(h / 24)}d${h % 24}h`;
  if (h > 0) return `${h}h${m}m`;
  return `${m}m`;
}

function pct(v: number): string {
  return (v * 100).toFixed(1) + "%";
}

function Bar({ value, color }: { value: number; color: string }) {
  return (
    <span className="stat-bar">
      <span className="stat-bar-fill" style={{ width: `${Math.min(100, Math.max(0, value))}%`, background: color }} />
    </span>
  );
}

function sr(s: UsageStats): number {
  return s.requests > 0 ? (s.successes / s.requests) * 100 : 0;
}

function srColor(v: number): string {
  return v >= 95 ? "var(--color-added)" : v >= 80 ? "var(--color-modified)" : "var(--color-deleted)";
}

export function Dashboard({ metrics, loading }: DashboardProps) {
  if (loading && !metrics) return <div className="dashboard-empty">加载中…</div>;
  if (!metrics) return <div className="dashboard-empty">未检测到 AMKR</div>;

  const t = metrics.total;
  const successRate = sr(t);
  const modelEntries = Object.entries(metrics.models);

  return (
    <div className="dashboard">
      {/* Row 1: Live metrics */}
      <div className="dash-row dash-headline">
        <div className="dash-stat">
          <AnimatedNumber value={fmt(t.requests)} className="dash-num" />
          <span className="dash-sub">总请求</span>
        </div>
        <div className="dash-stat">
          <span className="dash-num">{fmtUptime(metrics.started_at)}</span>
          <span className="dash-sub">运行</span>
        </div>
        <div className="dash-stat">
          <AnimatedNumber value={fmtMs(t.avg_first_token_ms)} className="dash-num" />
          <span className="dash-sub">首Token</span>
        </div>
        <div className="dash-stat">
          <AnimatedNumber value={fmtMs(t.avg_duration_ms)} className="dash-num" />
          <span className="dash-sub">平均延迟</span>
        </div>
      </div>

      {/* Row 2: Health bars */}
      <div className="dash-row dash-bars">
        <div className="dash-bar-item">
          <div className="dash-bar-head">
            <span className="dash-bar-label">成功率</span>
            <AnimatedNumber value={`${successRate.toFixed(1)}%`} className="dash-bar-value" />
          </div>
          <Bar value={successRate} color={srColor(successRate)} />
        </div>
        <div className="dash-bar-item">
          <div className="dash-bar-head">
            <span className="dash-bar-label">平均延迟</span>
            <AnimatedNumber value={fmtMs(t.avg_duration_ms)} className="dash-bar-value" />
          </div>
          <Bar value={Math.min(100, t.avg_duration_ms / 30)} color="var(--color-modified)" />
        </div>
        <div className="dash-bar-item">
          <div className="dash-bar-head">
            <span className="dash-bar-label">Token 缓存</span>
            <AnimatedNumber value={pct(t.cached_token_rate)} className="dash-bar-value" />
          </div>
          <Bar value={t.cached_token_rate * 100} color="var(--diff-hunk-color)" />
        </div>
      </div>

      {/* Row 3: Token breakdown */}
      <div className="dash-row dash-tokens">
        <div className="dash-token">
          <span className="dash-token-label">输入</span>
          <AnimatedNumber value={fmt(t.prompt_tokens)} className="dash-token-value" />
        </div>
        <div className="dash-token">
          <span className="dash-token-label">输出</span>
          <AnimatedNumber value={fmt(t.completion_tokens)} className="dash-token-value" />
        </div>
        <div className="dash-token">
          <span className="dash-token-label">缓存</span>
          <AnimatedNumber value={fmt(t.cached_tokens)} className="dash-token-value" />
        </div>
        <div className="dash-token">
          <span className="dash-token-label">总计</span>
          <AnimatedNumber value={fmt(t.total_tokens)} className="dash-token-value dash-token-total" />
        </div>
      </div>

      {/* Row 4: Latency details */}
      <div className="dash-row dash-latency">
        <div className="dash-latency-group">
          <span className="dash-latency-title">请求延迟</span>
          <span className="dash-latency-range">{fmtMs(t.min_duration_ms)} ~ {fmtMs(t.max_duration_ms)}</span>
        </div>
        <div className="dash-latency-group">
          <span className="dash-latency-title">首Token</span>
          <span className="dash-latency-range">{fmtMs(t.min_first_token_ms)} ~ {fmtMs(t.max_first_token_ms)}</span>
        </div>
        <div className="dash-latency-group">
          <span className="dash-latency-title">重试</span>
          <AnimatedNumber value={`${t.retries} 次`} className="dash-latency-range" />
        </div>
      </div>

      {/* Row 5: Per-model table */}
      {modelEntries.length > 0 && (
        <div className="dash-row dash-models">
          <table className="dash-table">
            <thead>
              <tr>
                <th>模型</th>
                <th>请求</th>
                <th>成功率</th>
                <th>延迟</th>
                <th>首Token</th>
                <th>Token</th>
              </tr>
            </thead>
            <tbody>
              {modelEntries.map(([name, s]) => (
                <tr key={name}>
                  <td className="dash-model-name">{name}</td>
                  <td><AnimatedNumber value={fmt(s.requests)} /></td>
                  <td style={{ color: srColor(sr(s)) }}><AnimatedNumber value={`${sr(s).toFixed(1)}%`} /></td>
                  <td><AnimatedNumber value={fmtMs(s.avg_duration_ms)} /></td>
                  <td><AnimatedNumber value={fmtMs(s.avg_first_token_ms)} /></td>
                  <td><AnimatedNumber value={fmt(s.total_tokens)} /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Row 6: Caller types */}
      {metrics.caller_types && Object.keys(metrics.caller_types).length > 1 && (
        <div className="dash-row dash-callers">
          {Object.entries(metrics.caller_types).map(([type, s]) => (
            <div key={type} className="dash-caller">
              <span className="dash-caller-type">{type}</span>
              <span className="dash-caller-stat">{fmt(s.requests)} 请求</span>
              <span className="dash-caller-stat">{fmt(s.total_tokens)} Token</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
