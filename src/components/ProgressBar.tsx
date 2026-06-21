/** Progress bar with colored fill */
export function ProgressBar({ value, color }: { value: number; color: string }) {
  return (
    <span className="stat-bar">
      <span className="stat-bar-fill" style={{ width: `${Math.min(100, Math.max(0, value))}%`, background: color }} />
    </span>
  );
}
