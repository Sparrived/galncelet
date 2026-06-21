import { AnimatedNumber } from "./AnimatedNumber";

interface StatCardProps {
  value: string;
  label: string;
  /** Wrap value in AnimatedNumber (default true) */
  animate?: boolean;
  /** Extra class on the number element */
  valueClassName?: string;
}

/** Compact stat display: number + label below */
export function StatCard({ value, label, animate = true, valueClassName }: StatCardProps) {
  const cls = `dash-num${valueClassName ? ` ${valueClassName}` : ""}`;
  return (
    <div className="dash-stat">
      {animate ? (
        <AnimatedNumber value={value} className={cls} />
      ) : (
        <span className={cls}>{value}</span>
      )}
      <span className="dash-sub">{label}</span>
    </div>
  );
}
