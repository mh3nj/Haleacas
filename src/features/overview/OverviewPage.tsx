import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../../lib/api";
import type { RepositoryFingerprint } from "../../types";
import { healthTone } from "../../components/HealthScore";

export function OverviewPage() {
  const [repos, setRepos] = useState<RepositoryFingerprint[] | null>(null);

  useEffect(() => {
    api.getRepositories().then(setRepos);
  }, []);

  if (repos === null) {
    return null;
  }

  if (repos.length === 0) {
    return (
      <div className="flex h-full flex-col items-start justify-center gap-3 px-8">
        <h1 className="text-lg font-semibold">No repositories indexed yet.</h1>
        <p className="max-w-md text-sm text-muted">
          Add a folder containing your projects and Haleacas will discover
          Git repositories automatically.
        </p>
        <Link
          to="/repositories"
          className="rounded bg-accent px-3 py-1.5 text-sm font-medium text-canvas hover:bg-accent-strong"
        >
          Add Repository Folder
        </Link>
      </div>
    );
  }

  const needsAttention = repos.filter((r) => healthTone(r.health.overall) !== "healthy");
  const critical = repos.filter((r) => healthTone(r.health.overall) === "critical");
  const sortedByHealth = [...repos].sort((a, b) => a.health.overall - b.health.overall);

  return (
    <div className="px-8 py-6">
      <h1 className="mb-4 text-lg font-semibold">Overview</h1>

      <div className="mb-6 grid grid-cols-3 gap-px overflow-hidden rounded border border-border bg-border">
        <Stat label="Repositories" value={repos.length} />
        <Stat label="Needs attention" value={needsAttention.length} tone="warning" />
        <Stat label="Critical" value={critical.length} tone="critical" />
      </div>

      <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
        Recent
      </h2>
      <div className="divide-y divide-border border-y border-border">
        {sortedByHealth.slice(0, 8).map((r) => (
          <Link
            key={r.id}
            to={`/repositories/${r.id}`}
            className="flex items-center justify-between px-2 py-2 text-sm hover:bg-surface-raised"
          >
            <span className="font-medium">{r.name}</span>
            <span className="flex items-center gap-3">
              <span className="text-muted">
                {!r.hasReadme
                  ? "README missing"
                  : !r.license
                    ? "No license detected"
                    : r.git.isDirty
                      ? "Uncommitted changes"
                      : "Looks tidy"}
              </span>
              <span className="font-mono text-2xs text-faint">
                {r.health.overall}/100
              </span>
            </span>
          </Link>
        ))}
      </div>
    </div>
  );
}

function Stat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone?: "healthy" | "warning" | "critical";
}) {
  const toneClass =
    tone === "healthy"
      ? "text-healthy"
      : tone === "warning"
        ? "text-warning"
        : tone === "critical"
          ? "text-critical"
          : "text-ink";
  return (
    <div className="bg-surface px-4 py-3">
      <div className={`font-mono text-2xl ${toneClass}`}>{value}</div>
      <div className="text-2xs uppercase tracking-wide text-faint">{label}</div>
    </div>
  );
}
