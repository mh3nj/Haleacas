import { useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../../lib/api";
import type { ActionsReport, GitHubWorkflowRun } from "../../types";
import { Badge } from "../../components/Badge";
import { formatDate } from "../../lib/format";

type Filter = "all" | "failed" | "running" | "passed" | "no-runs";

function runTone(run: GitHubWorkflowRun): "critical" | "healthy" | "warning" | "muted" {
  if (run.status !== "completed") return "warning";
  if (run.conclusion === "success") return "healthy";
  if (run.conclusion === "failure") return "critical";
  return "muted";
}

function runLabel(run: GitHubWorkflowRun): string {
  if (run.status !== "completed") return run.status.replace("_", " ");
  return run.conclusion ?? run.status;
}

function repoStatus(
  runs: GitHubWorkflowRun[],
): "failed" | "running" | "passed" | "no-runs" {
  if (runs.length === 0) return "no-runs";
  if (runs.some((r) => r.status === "completed" && r.conclusion === "failure"))
    return "failed";
  if (runs.some((r) => r.status !== "completed")) return "running";
  if (runs.some((r) => r.conclusion === "success")) return "passed";
  return "no-runs";
}

export function ActionsPage() {
  const [report, setReport] = useState<ActionsReport | null>(null);
  const [checking, setChecking] = useState(false);
  const [filter, setFilter] = useState<Filter>("all");

  const check = async () => {
    setChecking(true);
    try {
      setReport(await api.getGitHubActions());
    } finally {
      setChecking(false);
    }
  };

  const counts = { failed: 0, running: 0, passed: 0, noRuns: 0 };
  if (report) {
    for (const r of report.repos) {
      const s = repoStatus(r.runs);
      if (s === "failed") counts.failed += 1;
      else if (s === "running") counts.running += 1;
      else if (s === "passed") counts.passed += 1;
      else counts.noRuns += 1;
    }
  }

  const visible =
    report?.repos.filter((r) => {
      if (filter === "all") return true;
      return repoStatus(r.runs) === filter;
    }) ?? [];

  const filters: { key: Filter; label: string }[] = [
    { key: "all", label: "All" },
    { key: "failed", label: `Failing (${counts.failed})` },
    { key: "running", label: `Running (${counts.running})` },
    { key: "passed", label: `Passing (${counts.passed})` },
    { key: "no-runs", label: `No runs (${counts.noRuns})` },
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-3">
        <h1 className="text-lg font-semibold">GitHub Actions</h1>
        <p className="mt-1 max-w-2xl text-sm text-muted">
          Recent workflow runs for every local repo with a GitHub remote, in
          one place. This only checks when you ask, since each repo costs a
          slice of the GitHub request budget (60/hour without a token,
          5,000/hour with one connected in Settings).
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <button
            onClick={check}
            disabled={checking}
            className="rounded bg-accent px-3 py-1.5 text-sm font-medium text-canvas disabled:opacity-40"
          >
            {checking ? "Checking..." : "Check Actions"}
          </button>
          {report && (
            <div className="flex flex-wrap gap-1.5">
              {filters.map((f) => (
                <button
                  key={f.key}
                  onClick={() => setFilter(f.key)}
                  className={`rounded border px-2 py-1 text-xs ${
                    filter === f.key
                      ? "border-accent text-accent"
                      : "border-border text-muted hover:text-ink"
                  }`}
                >
                  {f.label}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {report === null ? (
          <p className="px-6 py-6 text-sm text-faint">
            Nothing checked yet this session. Hit Check Actions to pull the
            latest runs.
          </p>
        ) : report.reposWithGithub === 0 ? (
          <p className="px-6 py-6 text-sm text-faint">
            None of your indexed repos have a GitHub remote yet, so there is
            nothing to check.
          </p>
        ) : (
          <>
            <div className="flex flex-wrap gap-x-4 gap-y-1 border-b border-border px-6 py-2 font-mono text-2xs text-muted">
              <span>
                Checked {report.reposChecked} of {report.reposWithGithub} repos
                {report.truncated && !report.error ? " (capped)" : ""}
              </span>
              <span className="text-critical">{counts.failed} failing</span>
              <span className="text-warning">{counts.running} running</span>
              <span className="text-healthy">{counts.passed} passing</span>
            </div>
            {report.error && (
              <p className="border-b border-border px-6 py-2 text-sm text-critical">
                Stopped early: {report.error}
              </p>
            )}
            {visible.length === 0 ? (
              <p className="px-6 py-6 text-sm text-faint">
                No repos match this filter.
              </p>
            ) : (
              <div className="divide-y divide-border">
                {visible.map((r) => {
                  const status = repoStatus(r.runs);
                  return (
                    <div key={r.repoId} className="px-6 py-3">
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <div className="flex items-center gap-2">
                          <Link
                            to={`/repositories/${r.repoId}`}
                            className="text-sm font-semibold text-accent hover:underline"
                          >
                            {r.repoName}
                          </Link>
                          <Badge
                            tone={
                              status === "failed"
                                ? "critical"
                                : status === "running"
                                  ? "warning"
                                  : status === "passed"
                                    ? "healthy"
                                    : "muted"
                            }
                          >
                            {status === "no-runs"
                              ? "no runs"
                              : status}
                          </Badge>
                        </div>
                        <button
                          onClick={() =>
                            api.openUrl(`https://github.com/${r.owner}/${r.repo}/actions`)
                          }
                          className="text-2xs text-faint hover:text-accent"
                        >
                          Open Actions
                        </button>
                      </div>
                      {r.error ? (
                        <p className="mt-1 text-xs text-faint">{r.error}</p>
                      ) : r.runs.length === 0 ? (
                        <p className="mt-1 text-xs text-faint">
                          No workflow runs visible. The repo may have no
                          workflows, or they may be private.
                        </p>
                      ) : (
                        <div className="mt-2 space-y-1">
                          {r.runs.slice(0, 5).map((run, i) => (
                            <div
                              key={`${run.htmlUrl}-${i}`}
                              className="flex flex-wrap items-center justify-between gap-2 rounded border border-border px-2.5 py-1.5 text-xs"
                            >
                              <div className="flex min-w-0 items-center gap-2">
                                <span className="truncate font-medium">
                                  {run.name}
                                </span>
                                <Badge tone={runTone(run)}>
                                  {runLabel(run)}
                                </Badge>
                                {run.branch && (
                                  <span className="font-mono text-2xs text-faint">
                                    {run.branch}
                                  </span>
                                )}
                                {run.event && (
                                  <span className="font-mono text-2xs text-faint">
                                    {run.event}
                                  </span>
                                )}
                              </div>
                              <div className="flex items-center gap-2">
                                <span className="font-mono text-2xs text-faint">
                                  {formatDate(run.createdAt)}
                                </span>
                                <button
                                  onClick={() => api.openUrl(run.htmlUrl)}
                                  className="text-2xs text-faint hover:text-accent"
                                >
                                  Open
                                </button>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
