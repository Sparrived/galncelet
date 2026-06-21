/** Number with K/M suffix */
export function fmtNumber(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(1) + "K";
  return n.toLocaleString();
}

/** Milliseconds with s/ms suffix */
export function fmtMs(ms: number): string {
  if (ms >= 1000) return (ms / 1000).toFixed(1) + "s";
  return Math.round(ms) + "ms";
}

/** Bytes with B/KB/MB/GB suffix */
export function fmtBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(1) + " GB";
  if (bytes >= 1_048_576) return (bytes / 1_048_576).toFixed(1) + " MB";
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + " KB";
  return bytes + " B";
}

/** Hz with Hz/MHz/GHz suffix */
export function fmtHz(hz: number): string {
  if (hz >= 1_000_000_000) return (hz / 1_000_000_000).toFixed(2) + " GHz";
  if (hz >= 1_000_000) return (hz / 1_000_000).toFixed(0) + " MHz";
  if (hz >= 1000) return (hz / 1000).toFixed(0) + " KHz";
  return hz + " Hz";
}

/** Decimal to percent string (0.85 -> "85.0%") */
export function fmtPercent(v: number): string {
  return (v * 100).toFixed(1) + "%";
}

/** Uptime string from ISO timestamp */
export function fmtUptime(startedAt: string): string {
  const diff = Date.now() - new Date(startedAt).getTime();
  const h = Math.floor(diff / 3_600_000);
  const m = Math.floor((diff % 3_600_000) / 60_000);
  if (h > 24) return `${Math.floor(h / 24)}d${h % 24}h`;
  if (h > 0) return `${h}h${m}m`;
  return `${m}m`;
}
