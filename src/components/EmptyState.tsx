interface EmptyStateProps {
  message: string;
  icon?: React.ReactNode;
}

/** Centered placeholder for loading/empty/no-data states */
export function EmptyState({ message, icon }: EmptyStateProps) {
  return (
    <div className="dashboard-empty">
      {icon && <span style={{ marginBottom: 4 }}>{icon}</span>}
      {message}
    </div>
  );
}
