import type { RepositoryFingerprint } from "../../types";
import { Badge } from "../../components/Badge";
import { HealthScore, healthTone } from "../../components/HealthScore";
import { Link } from "react-router-dom";
import { formatSize, timeAgo } from "../../lib/format";

const toneToBg: Record<ReturnType<typeof healthTone>, string> = {
  healthy: "bg-healthy",
  warning: "bg-warning",
  critical: "bg-critical",
};

export function RepositoryRow({ repo }: { repo: RepositoryFingerprint }) {
  const statusColor = toneToBg[healthTone(repo.health.overall)];

  return (
    <div className="flex items-stretch gap-0 border-b border-border last:border-b-0">
      <div className={`w-1 ${statusColor}`} />
      <div className="flex flex-1 flex-col gap-1 px-3 py-2.5">
        <div className="flex items-baseline gap-2">
          <Link
            to={`/repositories/${repo.id}`}
            className="text-sm font-semibold hover:text-accent hover:underline"
          >
            {repo.name}
          </Link>
          {repo.git.branch && (
            <span className="font-mono text-2xs text-faint">
              {repo.git.branch}
            </span>
          )}
          {repo.description && (
            <span className="truncate text-xs text-muted">
              {repo.description}
            </span>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {repo.techs.slice(0, 6).map((t) => (
            <Badge key={t.name} tone="accent">
              {t.name}
            </Badge>
          ))}
          {!repo.hasReadme && <Badge tone="critical">No README</Badge>}
          {!repo.license && <Badge tone="warning">No license</Badge>}
          {!repo.hasTests && <Badge tone="muted">No tests detected</Badge>}
          {repo.git.isDirty && <Badge tone="warning">Uncommitted changes</Badge>}
        </div>

        <div className="flex gap-4 font-mono text-2xs text-faint">
          <span>{repo.approxLoc.toLocaleString()} LOC (approx.)</span>
          <span>{repo.fileCount.toLocaleString()} files</span>
          <span>{repo.git.commitCount.toLocaleString()} commits</span>
          <span>{formatSize(repo.sizeBytes)}</span>
          <span>{timeAgo(repo.git.lastCommitAt)}</span>
        </div>
      </div>
      <div className="flex w-16 shrink-0 flex-col items-center justify-center border-l border-border">
        <HealthScore value={repo.health.overall} size="sm" />
        <span className="text-2xs uppercase tracking-wide text-faint">health</span>
      </div>
    </div>
  );
}
