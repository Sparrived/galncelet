import type { AmkrMetrics, UsageStats } from "../lib/types";
import { fmtNumber as fmt, fmtMs, fmtUptime, fmtPercent as pct } from "../lib/format";
import { AnimatedNumber } from "./AnimatedNumber";
import { ProgressBar } from "./ProgressBar";
import { StatCard } from "./StatCard";
import { MetricRow } from "./MetricRow";

interface DashboardProps {
  metrics: AmkrMetrics | null;
  loading: boolean;
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
        <StatCard value={fmt(t.requests)} label="总请求" />
        <StatCard value={fmtUptime(metrics.started_at)} label="运行" animate={false} />
        <StatCard value={fmtMs(t.avg_first_token_ms)} label="首Token" />
        <StatCard value={fmtMs(t.avg_duration_ms)} label="平均延迟" />
      </div>

      {/* Row 2: Health bars */}
      <div className="dash-row dash-bars">
        <div className="dash-bar-item">
          <div className="dash-bar-head">
            <span className="dash-bar-label">成功率</span>
            <AnimatedNumber value={`${successRate.toFixed(1)}%`} className="dash-bar-value" />
          </div>
          <ProgressBar value={successRate} color={srColor(successRate)} />
        </div>
        <div className="dash-bar-item">
          <div className="dash-bar-head">
            <span className="dash-bar-label">平均延迟</span>
            <AnimatedNumber value={fmtMs(t.avg_duration_ms)} className="dash-bar-value" />
          </div>
          <ProgressBar value={Math.min(100, t.avg_duration_ms / 30)} color="var(--color-modified)" />
        </div>
        <div className="dash-bar-item">
          <div className="dash-bar-head">
            <span className="dash-bar-label">Token 缓存</span>
            <AnimatedNumber value={pct(t.cached_token_rate)} className="dash-bar-value" />
          </div>
          <ProgressBar value={t.cached_token_rate * 100} color="var(--diff-hunk-color)" />
        </div>
      </div>

      {/* Row 3: Token breakdown */}
      <MetricRow items={[
        { label: "输入", value: fmt(t.prompt_tokens) },
        { label: "输出", value: fmt(t.completion_tokens) },
        { label: "缓存", value: fmt(t.cached_tokens) },
        { label: "总计", value: fmt(t.total_tokens), isTotal: true },
      ]} />

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
