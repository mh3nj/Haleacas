import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { GitHubData, RepositoryFingerprint } from "../../types";
import { Badge } from "../../components/Badge";
import { formatDate } from "../../lib/format";

function Stat({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="border border-border bg-surface px-3 py-2">
      <div className="font-mono text-sm">{value}</div>
      <div className="text-2xs uppercase tracking-wide text-faint">{label}</div>
    </div>
  );
}

export function GitHubTab({ repo }: { repo: RepositoryFingerprint }) {
  const [data, setData] = useState<GitHubData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = () => {
    setLoading(true);
    setError(null);
    api
      .getGitHubData(repo.id)
      .then(setData)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    if (repo.git.githubSlug) load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repo.id]);

  if (!repo.git.githubSlug) {
    return (
      <p className="max-w-2xl text-sm text-faint">
        No GitHub remote was detected for this repository.
      </p>
    );
  }

  if (loading) return null;

  if (error) {
    return (
      <div className="max-w-2xl space-y-2">
        <p className="text-sm text-critical">{error}</p>
        <button
          onClick={load}
          className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
        >
          Retry
        </button>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="max-w-2xl space-y-4">
      <p className="text-2xs text-faint">
        Fetched through the GitHub API. Connect a token in Settings for private repo access
        and a higher rate limit.
      </p>

      <div className="flex flex-wrap gap-1.5">
        <Badge tone="muted">{data.visibility}</Badge>
        {data.archived && <Badge tone="warning">Archived</Badge>}
        <Badge tone="muted">{data.defaultBranch}</Badge>
      </div>

      <div className="grid grid-cols-4 gap-px overflow-hidden border border-border bg-border">
        <Stat label="Stars" value={data.stars.toLocaleString()} />
        <Stat label="Forks" value={data.forks.toLocaleString()} />
        <Stat
          label="Open PRs"
          value={`${data.openPullRequests}${data.openPullRequestsTruncated ? "+" : ""}`}
        />
        <Stat label="Issues + PRs (open)" value={data.openIssuesAndPrs.toLocaleString()} />
      </div>

      <section>
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Latest release
        </h2>
        {data.latestRelease ? (
          <div className="border border-border px-3 py-2 text-sm">
            <div>{data.latestRelease.name || data.latestRelease.tagName}</div>
            <div className="mt-1 font-mono text-2xs text-faint">
              {data.latestRelease.tagName}
              {data.latestRelease.publishedAt && ` · ${formatDate(data.latestRelease.publishedAt)}`}
            </div>
          </div>
        ) : (
          <p className="text-sm text-faint">No releases.</p>
        )}
      </section>

      <section>
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Latest CI run
        </h2>
        {data.latestWorkflowRun ? (
          <div className="border border-border px-3 py-2 text-sm">
            <div className="flex items-center gap-2">
              <span>{data.latestWorkflowRun.name}</span>
              <Badge
                tone={
                  data.latestWorkflowRun.conclusion === "success"
                    ? "healthy"
                    : data.latestWorkflowRun.conclusion === "failure"
                      ? "critical"
                      : "muted"
                }
              >
                {data.latestWorkflowRun.conclusion || data.latestWorkflowRun.status}
              </Badge>
            </div>
            <div className="mt-1 font-mono text-2xs text-faint">
              {formatDate(data.latestWorkflowRun.createdAt)}
            </div>
          </div>
        ) : (
          <p className="text-sm text-faint">No workflow runs visible.</p>
        )}
      </section>

      <button
        onClick={() => api.openUrl(data.htmlUrl)}
        className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
      >
        Open on GitHub
      </button>
    </div>
  );
}
