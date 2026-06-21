import { useEffect, useRef, useState } from "react";

/** 数字变化时带闪烁高亮效果的组件 */
export function AnimatedNumber({ value, className }: { value: string; className?: string }) {
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
