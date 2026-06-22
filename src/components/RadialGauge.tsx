interface RadialGaugeProps {
  /** 0-100 */
  value: number;
  label: string;
  color?: string;
  size?: number;
  stroke?: number;
  sub?: string;
  /** Second line (e.g. temperature) */
  sub2?: string;
  /** Color for sub2 text */
  sub2Color?: string;
}

/** Circular SVG gauge — uses stroke-dashoffset for smooth animation */
export function RadialGauge({
  value,
  label,
  color = "var(--mcha-cyan)",
  size = 56,
  stroke = 4,
  sub,
  sub2,
  sub2Color,
}: RadialGaugeProps) {
  const center = size / 2;
  const radius = (size - stroke) / 2 - 1;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.max(0, Math.min(100, value));
  const offset = circumference * (1 - clamped / 100);

  // Push main label up when sub lines exist
  const labelY = sub2 ? center - 4 : sub ? center - 2 : center + 1;

  return (
    <svg
      className="rg-svg"
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      style={{ "--rg-color": color } as React.CSSProperties}
    >
      <circle
        className="rg-track"
        cx={center}
        cy={center}
        r={radius}
        fill="none"
        strokeWidth={stroke}
      />
      <circle
        className="rg-fill"
        cx={center}
        cy={center}
        r={radius}
        fill="none"
        strokeWidth={stroke}
        strokeDasharray={circumference}
        strokeDashoffset={offset}
      />
      <text
        className="rg-label"
        x={center}
        y={labelY}
        textAnchor="middle"
        dominantBaseline="central"
      >
        {label}
      </text>
      {sub && (
        <text
          className="rg-sub"
          x={center}
          y={center + 6}
          textAnchor="middle"
          dominantBaseline="central"
        >
          {sub}
        </text>
      )}
      {sub2 && (
        <text
          className="rg-sub"
          x={center}
          y={center + 15}
          textAnchor="middle"
          dominantBaseline="central"
          style={sub2Color ? { fill: sub2Color } : undefined}
        >
          {sub2}
        </text>
      )}
    </svg>
  );
}
