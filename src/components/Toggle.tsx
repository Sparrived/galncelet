interface ToggleProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  title?: string;
  disabled?: boolean;
}

/** Toggle switch (on/off) */
export function Toggle({ checked, onChange, title, disabled }: ToggleProps) {
  return (
    <button
      className={`toggle${checked ? " toggle-on" : ""}`}
      onClick={() => !disabled && onChange(!checked)}
      title={title}
      disabled={disabled}
    >
      <div className="toggle-knob" />
    </button>
  );
}
