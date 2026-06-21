import { AnimatedNumber } from "./AnimatedNumber";

interface MetricItem {
  label: string;
  value: string;
  isTotal?: boolean;
}

interface MetricRowProps {
  items: MetricItem[];
  animate?: boolean;
}

/** Horizontal row of label+value metric cards */
export function MetricRow({ items, animate = true }: MetricRowProps) {
  return (
    <div className="dash-tokens">
      {items.map((item) => (
        <div key={item.label} className="dash-token">
          <span className="dash-token-label">{item.label}</span>
          {animate ? (
            <AnimatedNumber
              value={item.value}
              className={`dash-token-value${item.isTotal ? " dash-token-total" : ""}`}
            />
          ) : (
            <span className={`dash-token-value${item.isTotal ? " dash-token-total" : ""}`}>
              {item.value}
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
