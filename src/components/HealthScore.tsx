export function healthTone(score: number): "healthy" | "warning" | "critical" {
  if (score >= 80) return "healthy";
  if (score >= 50) return "warning";
  return "critical";
}

const toneTextClass: Record<ReturnType<typeof healthTone>, string> = {
  healthy: "text-healthy",
  warning: "text-warning",
  critical: "text-critical",
};

export function HealthScore({
  value,
  size = "md",
}: {
  value: number;
  size?: "sm" | "md";
}) {
  const tone = healthTone(value);
  const sizeClass = size === "sm" ? "text-sm" : "text-2xl";
  return (
    <span className={`font-mono ${sizeClass} ${toneTextClass[tone]}`}>
      {value}
    </span>
  );
}
