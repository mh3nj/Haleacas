interface BadgeProps {
  children: React.ReactNode;
  tone?: "muted" | "accent" | "healthy" | "warning" | "critical";
}

const toneClasses: Record<NonNullable<BadgeProps["tone"]>, string> = {
  muted: "text-muted border-border",
  accent: "text-accent border-accent/40",
  healthy: "text-healthy border-healthy/40",
  warning: "text-warning border-warning/40",
  critical: "text-critical border-critical/40",
};

export function Badge({ children, tone = "muted" }: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center rounded-sm border px-1.5 py-0.5 text-2xs font-mono uppercase tracking-wide ${toneClasses[tone]}`}
    >
      {children}
    </span>
  );
}
