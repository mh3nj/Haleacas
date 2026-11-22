import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { api } from "../../lib/api";
import type { DoctorFinding, RepositoryFingerprint } from "../../types";
import { Badge } from "../../components/Badge";
import { HealthScore } from "../../components/HealthScore";
import { FixesPanel } from "./FixesPanel";
import { SecurityFindingsPanel } from "./SecurityFindingsPanel";
import { ExplorerTab } from "./ExplorerTab";
import { DependenciesTab } from "./DependenciesTab";
import { ReadmeTab } from "./ReadmeTab";
import { TimelineTab } from "./TimelineTab";
import { GitHubTab } from "./GitHubTab";
import { AITab } from "./AITab";
import { formatSize, formatDate } from "../../lib/format";

const TABS = [
  { key: "overview", label: "Overview", shipped: true },
  { key: "health", label: "Health", shipped: true },
  { key: "files", label: "Files", shipped: true },
  { key: "dependencies", label: "Dependencies", shipped: true },
  { key: "readme", label: "README", shipped: true },
  { key: "timeline", label: "Timeline", shipped: true },
  { key: "github", label: "GitHub", shipped: true },
  { key: "ai", label: "AI", shipped: true },
] as const;

type TabKey = (typeof TABS)[number]["key"];

export function RepositoryDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const [repo, setRepo] = useState<RepositoryFingerprint | null | undefined>(
    undefined
  );
  const [tab, setTab] = useState<TabKey>("overview");

  useEffect(() => {
    if (!id) return;
    api.getRepository(id).then(setRepo);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  // Keep the tab in sync with `?tab=` separately from the repo fetch
  // above, so Command Center deep-links (Health/README/Timeline/AI for
  // the already-open repository) actually switch tabs instead of being
  // ignored because the id didn't change.
  useEffect(() => {
    const requested = searchParams.get("tab");
    setTab(TABS.some((t) => t.key === requested) ? (requested as TabKey) : "overview");
  }, [searchParams]);

  const refreshRepo = () => {
    if (id) api.getRepository(id).then(setRepo);
  };

  if (repo === undefined) return null;

  if (repo === null) {
    return (
      <div className="flex h-full flex-col items-start justify-center gap-2 px-8">
        <h1 className="text-lg font-semibold">Repository not found.</h1>
        <p className="max-w-md text-sm text-muted">
          It may not have been scanned yet, or its folder was removed from
          Settings.
        </p>
        <Link to="/repositories" className="text-sm text-accent hover:underline">
          Back to Repositories
        </Link>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-4">
        <Link
          to="/repositories"
          className="mb-2 inline-block text-2xs text-faint hover:text-muted"
        >
          ← Repositories
        </Link>
        <div className="flex items-start justify-between gap-4">
          <div>
            <div className="flex items-baseline gap-2">
              <h1 className="text-lg font-semibold">{repo.name}</h1>
              <HealthScore value={repo.health.overall} size="sm" />
              {repo.git.branch && (
                <span className="font-mono text-xs text-faint">
                  {repo.git.branch}
                </span>
              )}
              {(repo.git.ahead > 0 || repo.git.behind > 0) && (
                <span className="font-mono text-2xs text-muted">
                  {repo.git.ahead > 0 && `↑${repo.git.ahead}`}
                  {repo.git.behind > 0 && ` ↓${repo.git.behind}`}
                </span>
              )}
            </div>
            {repo.description && (
              <p className="mt-1 text-sm text-muted">{repo.description}</p>
            )}
            <div className="mt-2 flex flex-wrap gap-1.5">
              {repo.techs.map((t) => (
                <Badge key={t.name} tone="accent">
                  <span title={t.evidence}>{t.name}</span>
                </Badge>
              ))}
              {repo.git.isDirty && (
                <Badge tone="warning">Uncommitted changes</Badge>
              )}
              {!repo.hasReadme && <Badge tone="critical">No README</Badge>}
              {!repo.license && <Badge tone="warning">No license</Badge>}
            </div>
          </div>

          <div className="flex shrink-0 gap-2">
            <button
              onClick={() => api.openInFileManager(repo.path)}
              className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
            >
              Open Folder
            </button>
            <button
              onClick={() => api.openInEditor(repo.path)}
              className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
            >
              Open in Editor
            </button>
            {repo.git.githubSlug && (
              <button
                onClick={() =>
                  api.openUrl(`https://github.com/${repo.git.githubSlug}`)
                }
                className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
              >
                Open GitHub
              </button>
            )}
          </div>
        </div>
      </div>

      <div className="flex gap-1 border-b border-border px-6">
        {TABS.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`flex items-center gap-1.5 border-b-2 px-2 py-2 text-sm ${
              tab === t.key
                ? "border-accent text-ink"
                : "border-transparent text-muted hover:text-ink"
            }`}
          >
            {t.label}
            {!t.shipped && <Badge>Soon</Badge>}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        {tab === "overview" ? (
          <OverviewTab repo={repo} />
        ) : tab === "health" ? (
          <HealthTab repo={repo} onRepoUpdated={refreshRepo} />
        ) : tab === "files" ? (
          <ExplorerTab repo={repo} />
        ) : tab === "dependencies" ? (
          <DependenciesTab repo={repo} />
        ) : tab === "readme" ? (
          <ReadmeTab repo={repo} onRepoUpdated={refreshRepo} />
        ) : tab === "timeline" ? (
          <TimelineTab repo={repo} />
        ) : tab === "github" ? (
          <GitHubTab repo={repo} />
        ) : tab === "ai" ? (
          <AITab repo={repo} />
        ) : (
          <div className="pt-8 text-sm text-muted">
            This tab is not built yet. It is on the roadmap for a later
            development phase.
          </div>
        )}
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="border border-border bg-surface px-3 py-2">
      <div className="font-mono text-sm">{value}</div>
      <div className="text-2xs uppercase tracking-wide text-faint">{label}</div>
    </div>
  );
}

function OverviewTab({ repo }: { repo: RepositoryFingerprint }) {
  return (
    <div className="max-w-2xl space-y-6">
      <div className="grid grid-cols-4 gap-px overflow-hidden border border-border bg-border">
        <Stat label="Approx. LOC" value={repo.approxLoc.toLocaleString()} />
        <Stat label="Files" value={repo.fileCount.toLocaleString()} />
        <Stat label="Commits" value={repo.git.commitCount.toLocaleString()} />
        <Stat label="Size" value={formatSize(repo.sizeBytes)} />
      </div>

      <section>
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Last commit
        </h2>
        {repo.git.lastCommitHash ? (
          <div className="border border-border px-3 py-2 text-sm">
            <div>{repo.git.lastCommitMessage || "(no message)"}</div>
            <div className="mt-1 font-mono text-2xs text-faint">
              {repo.git.lastCommitHash.slice(0, 8)} · {repo.git.lastCommitAuthor || "unknown"} ·{" "}
              {formatDate(repo.git.lastCommitAt)}
            </div>
          </div>
        ) : (
          <p className="text-sm text-faint">No commits yet.</p>
        )}
      </section>

      <section>
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Working tree {repo.git.isDirty ? "" : "(clean)"}
        </h2>
        {repo.git.changedFiles.length === 0 ? (
          <p className="text-sm text-faint">No uncommitted changes.</p>
        ) : (
          <div className="divide-y divide-border border border-border">
            {repo.git.changedFiles.map((f) => (
              <div
                key={f.path}
                className="flex items-center justify-between px-3 py-1.5 text-sm"
              >
                <span className="font-mono text-xs">{f.path}</span>
                <span className="text-2xs uppercase tracking-wide text-faint">
                  {f.status}
                </span>
              </div>
            ))}
            {repo.git.changedFilesTruncated && (
              <div className="px-3 py-1.5 text-2xs text-faint">
                List truncated. More files are changed than shown here.
              </div>
            )}
          </div>
        )}
      </section>

      <section>
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Detected
        </h2>
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div>License: {repo.license || "not detected"}</div>
          <div>README: {repo.hasReadme ? "present" : "missing"}</div>
          <div>.gitignore: {repo.hasGitignore ? "present" : "missing"}</div>
          <div>CI config: {repo.hasCi ? "present" : "missing"}</div>
          <div>Tests directory: {repo.hasTests ? "present" : "missing"}</div>
        </div>
      </section>
    </div>
  );
}

const severityTone: Record<DoctorFinding["severity"], "critical" | "warning" | "muted"> = {
  critical: "critical",
  warning: "warning",
  improvement: "muted",
  info: "muted",
};

function HealthTab({
  repo,
  onRepoUpdated,
}: {
  repo: RepositoryFingerprint;
  onRepoUpdated: () => void;
}) {
  const [findings, setFindings] = useState<DoctorFinding[] | null>(null);

  const refreshFindings = () => {
    api.getDoctorFindings(repo.id).then(setFindings);
  };

  useEffect(() => {
    refreshFindings();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repo.id]);

  return (
    <div className="max-w-2xl space-y-6">
      <div className="flex items-center gap-3 border border-border px-4 py-3">
        <HealthScore value={repo.health.overall} />
        <div className="text-sm text-muted">
          The Haleacas Health Score is based on the checks below, not an
          industry-standard metric.
        </div>
      </div>

      <section className="space-y-3">
        <h2 className="font-mono text-2xs uppercase tracking-wide text-faint">
          Categories
        </h2>
        {repo.health.categories.map((cat) => (
          <div key={cat.category} className="border border-border">
            <div className="flex items-center justify-between border-b border-border px-3 py-1.5">
              <span className="text-sm font-medium">{cat.category}</span>
              <span className="font-mono text-sm">{cat.score}</span>
            </div>
            <div className="divide-y divide-border">
              {cat.checks.map((check) => (
                <div key={check.label} className="px-3 py-1.5 text-sm">
                  <div className="flex items-center gap-2">
                    <span className={check.passed ? "text-healthy" : "text-critical"}>
                      {check.passed ? "+" : "−"}
                    </span>
                    <span>{check.label}</span>
                  </div>
                  <div className="pl-4 text-2xs text-faint">{check.detail}</div>
                </div>
              ))}
            </div>
          </div>
        ))}
        {repo.health.notYetScored.length > 0 && (
          <p className="text-2xs text-faint">
            Not yet scored: {repo.health.notYetScored.join(", ")}. These
            depend on features from later development phases (dependency
            parsing, secret scanning).
          </p>
        )}
      </section>

      <section>
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Repository Doctor
        </h2>
        {findings === null ? null : findings.length === 0 ? (
          <p className="text-sm text-faint">No findings.</p>
        ) : (
          <div className="space-y-2">
            {findings.map((f) => (
              <div key={f.title} className="border border-border px-3 py-2">
                <div className="flex items-center gap-2">
                  <Badge tone={severityTone[f.severity]}>{f.severity}</Badge>
                  <span className="text-sm font-medium">{f.title}</span>
                </div>
                <p className="mt-1 text-sm text-muted">{f.explanation}</p>
                <p className="mt-1 text-2xs text-faint">
                  Why it matters: {f.whyItMatters}
                </p>
                <p className="mt-1 text-2xs">
                  <span className="text-faint">Recommended: </span>
                  {f.recommendedAction}
                </p>
                {f.affectedFiles.length > 0 && (
                  <p className="mt-1 font-mono text-2xs text-faint">
                    {f.affectedFiles.join(", ")}
                  </p>
                )}
              </div>
            ))}
          </div>
        )}
      </section>

      <SecurityFindingsPanel
        repo={repo}
        onChanged={() => {
          onRepoUpdated();
          refreshFindings();
        }}
      />

      <FixesPanel
        repoId={repo.id}
        onApplied={() => {
          onRepoUpdated();
          refreshFindings();
        }}
      />
    </div>
  );
}
