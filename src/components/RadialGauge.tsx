interface RadialGaugeProps {
  /** 0-100 */
  value: number;
  label: string;
  color?: string;
  size?: number;
  stroke?: number;
  sub?: string;
}

/** Convert polar to SVG arc path (clockwise from top) */
function arcPath(cx: number, cy: number, r: number, pct: number): string {
  if (pct <= 0) return "";
  const angle = Math.min(pct / 100, 0.999) * 2 * Math.PI;
  const startAngle = -Math.PI / 2;
  const endAngle = startAngle + angle;
  const x1 = cx + r * Math.cos(startAngle);
  const y1 = cy + r * Math.sin(startAngle);
  const x2 = cx + r * Math.cos(endAngle);
  const y2 = cy + r * Math.sin(endAngle);
  const large = angle > Math.PI ? 1 : 0;
  return `M ${x1} ${y1} A ${r} ${r} 0 ${large} 1 ${x2} ${y2}`;
}

/** Circular SVG gauge — mecha style, text centered inside ring */
export function RadialGauge({
  value,
  label,
  color = "var(--mcha-cyan)",
  size = 80,
  stroke = 6,
  sub,
}: RadialGaugeProps) {
  const center = size / 2;
  const radius = (size - stroke) / 2 - 1;
  const clamped = Math.max(0, Math.min(100, value));

  return (
    <svg
      className="rg-svg"
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      style={{ "--rg-color": color } as React.CSSProperties}
    >
      {/* Track ring */}
      <circle
        className="rg-track"
        cx={center}
        cy={center}
        r={radius}
        fill="none"
        strokeWidth={stroke}
      />
      {/* Value arc */}
      <path
        className="rg-fill"
        d={arcPath(center, center, radius, clamped)}
        fill="none"
        strokeWidth={stroke}
        strokeLinecap="round"
      />
      {/* Center percentage */}
      <text
        className="rg-label"
        x={center}
        y={sub ? center - 3 : center + 1}
        textAnchor="middle"
        dominantBaseline="central"
      >
        {label}
      </text>
      {/* Sub label */}
      {sub && (
        <text
          className="rg-sub"
          x={center}
          y={center + 11}
          textAnchor="middle"
          dominantBaseline="central"
        >
          {sub}
        </text>
      )}
    </svg>
  );
}
