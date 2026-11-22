import { useEffect, useMemo, useState } from "react";
import { api } from "../../lib/api";
import type {
  GitHubRelease,
  RepositoryFingerprint,
  TimelineEntry,
  TimelineReport,
  TreeStats,
} from "../../types";
import { Badge } from "../../components/Badge";
import { formatDate } from "../../lib/format";

function monthKey(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleString(undefined, { year: "numeric", month: "short" });
}

function groupByMonth(entries: TimelineEntry[]): [string, TimelineEntry[]][] {
  const groups = new Map<string, TimelineEntry[]>();
  for (const entry of entries) {
    const key = monthKey(entry.date);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(entry);
  }
  return Array.from(groups.entries());
}

function StatsCard({ title, stats }: { title: string; stats: TreeStats | null }) {
  return (
    <div className="flex-1 border border-border p-3">
      <h4 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">{title}</h4>
      {stats === null ? (
        <p className="text-sm text-faint">Select a commit.</p>
      ) : (
        <div className="space-y-1 text-sm">
          <div>{stats.fileCount.toLocaleString()} files</div>
          <div>{stats.approxLoc.toLocaleString()} LOC (approx.)</div>
          <div>{stats.hasTests ? "Has tests" : "No tests"}</div>
          <div>{stats.hasCi ? "Has CI" : "No CI"}</div>
          <div>{stats.hasReadme ? "Has README" : "No README"}</div>
          <div>{stats.hasGitignore ? "Has .gitignore" : "No .gitignore"}</div>
        </div>
      )}
    </div>
  );
}

export function TimelineTab({ repo }: { repo: RepositoryFingerprint }) {
  const [report, setReport] = useState<TimelineReport | null>(null);
  const [fromHash, setFromHash] = useState<string>("");
  const [toHash, setToHash] = useState<string>("");
  const [fromStats, setFromStats] = useState<TreeStats | null>(null);
  const [toStats, setToStats] = useState<TreeStats | null>(null);
  const [releases, setReleases] = useState<GitHubRelease[]>([]);
  const [releasesTruncated, setReleasesTruncated] = useState(false);
  const [releasesError, setReleasesError] = useState<string | null>(null);

  useEffect(() => {
    api.getRepositoryTimeline(repo.id).then((r) => {
      setReport(r);
      if (r.entries.length > 0) {
        setFromHash(r.entries[0].hash);
        setToHash(r.entries[r.entries.length - 1].hash);
      }
    });
  }, [repo.id]);

  useEffect(() => {
    if (fromHash) api.getStatsAtCommit(repo.id, fromHash).then(setFromStats);
  }, [repo.id, fromHash]);

  useEffect(() => {
    if (toHash) api.getStatsAtCommit(repo.id, toHash).then(setToStats);
  }, [repo.id, toHash]);

  // Releases are an optional online enrichment of an otherwise fully
  // offline timeline — fetched automatically (same precedent as
  // GitHubTab's single-repo fetch; unlike the multi-repo Inbox check,
  // one repository's worth of requests isn't worth gating behind an
  // extra click), and a failure here never blocks the commit history
  // itself from rendering.
  useEffect(() => {
    setReleases([]);
    setReleasesTruncated(false);
    setReleasesError(null);
    if (!repo.git.githubSlug) return;
    api
      .getRepositoryReleases(repo.id)
      .then((r) => {
        setReleases(r.releases);
        setReleasesTruncated(r.truncated);
      })
      .catch((e) => setReleasesError(String(e)));
  }, [repo.id, repo.git.githubSlug]);

  const releaseByTag = useMemo(() => {
    const map = new Map<string, GitHubRelease>();
    for (const r of releases) map.set(r.tagName, r);
    return map;
  }, [releases]);

  const grouped = useMemo(() => (report ? groupByMonth(report.entries) : []), [report]);

  if (report === null) return null;

  if (report.entries.length === 0) {
    return <p className="text-sm text-faint">No commits found.</p>;
  }

  return (
    <div className="max-w-4xl space-y-4">
      {report.truncated && (
        <p className="text-2xs text-faint">
          Showing the most recent commits. This repo has more history than fits in one view.
        </p>
      )}
      {releasesTruncated && (
        <p className="text-2xs text-faint">
          This repo has more GitHub releases than fit in one check, so only the most recent
          ones are matched against tags below.
        </p>
      )}
      {releasesError && (
        <p className="text-2xs text-faint">Couldn&rsquo;t check GitHub releases: {releasesError}</p>
      )}

      <div className="flex gap-3">
        <StatsCard title="Before" stats={fromStats} />
        <StatsCard title="After" stats={toStats} />
      </div>

      <div className="max-h-96 overflow-y-auto border border-border">
        {grouped.map(([month, entries]) => (
          <div key={month}>
            <div className="sticky top-0 bg-surface-raised px-2 py-1 font-mono text-2xs uppercase tracking-wide text-faint">
              {month}
            </div>
            {entries.map((entry) => (
              <div
                key={entry.hash}
                className={`flex items-center justify-between border-b border-border px-2 py-1.5 text-sm ${
                  entry.hash === toHash ? "bg-surface-raised" : ""
                }`}
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate">{entry.message || "(no message)"}</span>
                    {entry.tags.map((t) => {
                      const release = releaseByTag.get(t);
                      return release ? (
                        <button
                          key={t}
                          onClick={() => api.openUrl(release.htmlUrl)}
                          title={`Open ${release.name || release.tagName} on GitHub`}
                        >
                          <Badge tone="accent">{t} ↗</Badge>
                        </button>
                      ) : (
                        <Badge key={t} tone="accent">
                          {t}
                        </Badge>
                      );
                    })}
                  </div>
                  <div className="font-mono text-2xs text-faint">
                    {entry.shortHash} · {entry.author} · {formatDate(entry.date)}
                  </div>
                </div>
                <div className="flex shrink-0 gap-1">
                  <button
                    onClick={() => setFromHash(entry.hash)}
                    className="text-2xs text-faint hover:text-accent"
                  >
                    Set Before
                  </button>
                  <button
                    onClick={() => setToHash(entry.hash)}
                    className="text-2xs text-faint hover:text-accent"
                  >
                    Set After
                  </button>
                </div>
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
