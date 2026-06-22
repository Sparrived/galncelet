interface RadialGaugeProps {
  /** 0-100 */
  value: number;
  label: string;
  color?: string;
  size?: number;
  stroke?: number;
  sub?: string;
}

/** Circular SVG gauge — uses stroke-dashoffset for smooth animation */
export function RadialGauge({
  value,
  label,
  color = "var(--mcha-cyan)",
  size = 56,
  stroke = 4,
  sub,
}: RadialGaugeProps) {
  const center = size / 2;
  const radius = (size - stroke) / 2 - 1;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.max(0, Math.min(100, value));
  const offset = circumference * (1 - clamped / 100);

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
        y={sub ? center - 2 : center + 1}
        textAnchor="middle"
        dominantBaseline="central"
      >
        {label}
      </text>
      {sub && (
        <text
          className="rg-sub"
          x={center}
          y={center + 8}
          textAnchor="middle"
          dominantBaseline="central"
        >
          {sub}
        </text>
      )}
    </svg>
  );
}
