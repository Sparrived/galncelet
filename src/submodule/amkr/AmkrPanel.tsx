import { useEffect, useState, useRef, useCallback } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { fetchAmkrMetrics, loadSettings } from "../../lib/api";
import type { AmkrMetrics } from "../../lib/types";
import { Dashboard } from "../../components/Dashboard";

/** WidgetShell header height — must match WidgetShell.HEADER_H */
const WIDGET_HEADER_H = 36;

function fmt(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(1) + "K";
  return n.toLocaleString();
}

export default function AmkrPanel() {
  const [metrics, setMetrics] = useState<AmkrMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Resize the window to fit content (only adjust height, keep width from settings)
  const fitToContent = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    const contentH = el.scrollHeight;
    const totalH = contentH + WIDGET_HEADER_H;
    loadSettings().then((s) => {
      const win = getCurrentWindow();
      win.setSize(new LogicalSize(s.cardWidth, Math.max(totalH, 100)));
    }).catch(() => {});
  }, []);

  // Observe content size changes
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver(() => {
      requestAnimationFrame(fitToContent);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [fitToContent]);

  // Probe once on mount
  useEffect(() => {
    fetchAmkrMetrics()
      .then((data) => { if (data) setMetrics(data); })
      .catch(() => {});
  }, []);

  // Poll every 10 seconds
  useEffect(() => {
    const poll = async () => {
      setLoading(true);
      try {
        const data = await fetchAmkrMetrics();
        setMetrics(data);
      } catch {
        setMetrics(null);
      } finally {
        setLoading(false);
      }
    };
    const id = setInterval(poll, 10000);
    return () => clearInterval(id);
  }, []);

  return (
    <div ref={containerRef} className="dashboard-body">
      {/* RPM + TPM badge positioned over the widget header */}
      {metrics && (
        <span className="amkr-rpm-badge"><span className="amkr-rpm">{metrics.current_rpm} rpm</span> · <span className="amkr-tpm">{fmt(metrics.current_tpm)} tpm</span></span>
      )}
      <Dashboard metrics={metrics} loading={loading} />
    </div>
  );
}
