import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../../lib/api";
import type { PagesReport } from "../../types";
import { Badge } from "../../components/Badge";

type Filter = "all" | "ready" | "missing";

export function PagesPage() {
  const [report, setReport] = useState<PagesReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [creating, setCreating] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      setReport(await api.getPagesReport());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const createIndex = async (repoId: string) => {
    setCreating(repoId);
    try {
      setReport(await api.createPagesIndex(repoId));
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(null);
    }
  };

  const visible =
    report?.repos.filter((r) => {
      if (filter === "all") return true;
      if (filter === "ready") return r.ready;
      return !r.ready;
    }) ?? [];

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-3">
        <h1 className="text-lg font-semibold">GitHub Pages</h1>
        <p className="mt-1 max-w-2xl text-sm text-muted">
          Checks every local repo for an entry page a static host can serve.
          If an index.html is there, you are good to publish. If not, you
          will see exactly which repo needs one. This runs fully offline.
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <button
            onClick={load}
            disabled={loading}
            className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised disabled:opacity-40"
          >
            {loading ? "Checking..." : "Re-check"}
          </button>
          {report && (
            <div className="flex flex-wrap gap-1.5">
              {(
                [
                  { key: "all", label: `All (${report.total})` },
                  { key: "ready", label: `Ready (${report.ready})` },
                  { key: "missing", label: `Missing (${report.missing})` },
                ] as { key: Filter; label: string }[]
              ).map((f) => (
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
        {loading && report === null ? (
          <p className="px-6 py-6 text-sm text-faint">Checking local files...</p>
        ) : error && report === null ? (
          <div className="space-y-2 px-6 py-6">
            <p className="text-sm text-critical">{error}</p>
            <button
              onClick={load}
              className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
            >
              Retry
            </button>
          </div>
        ) : report === null || report.total === 0 ? (
          <p className="px-6 py-6 text-sm text-faint">
            No repos indexed yet. Add a folder on the Repositories page first.
          </p>
        ) : (
          <>
            <div className="flex flex-wrap gap-x-4 border-b border-border px-6 py-2 font-mono text-2xs text-muted">
              <span className="text-healthy">{report.ready} ready</span>
              <span className="text-warning">{report.missing} missing</span>
            </div>
            {error && (
              <p className="border-b border-border px-6 py-2 text-sm text-critical">
                {error}
              </p>
            )}
            {visible.length === 0 ? (
              <p className="px-6 py-6 text-sm text-faint">
                No repos match this filter.
              </p>
            ) : (
              <div className="divide-y divide-border">
                {visible.map((r) => (
                  <div key={r.repoId} className="px-6 py-3">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div className="flex min-w-0 items-center gap-2">
                        <Link
                          to={`/repositories/${r.repoId}`}
                          className="truncate text-sm font-semibold text-accent hover:underline"
                        >
                          {r.repoName}
                        </Link>
                        <Badge tone={r.ready ? "healthy" : "warning"}>
                          {r.ready ? "ready" : "missing index.html"}
                        </Badge>
                        {r.hasCname && <Badge tone="muted">CNAME</Badge>}
                        {r.hasNojekyll && <Badge tone="muted">.nojekyll</Badge>}
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => api.openInFileManager(r.path)}
                          className="text-2xs text-faint hover:text-accent"
                        >
                          Open folder
                        </button>
                        {!r.ready && (
                          <button
                            onClick={() => createIndex(r.repoId)}
                            disabled={creating === r.repoId}
                            className="rounded border border-border px-2 py-0.5 text-2xs hover:bg-surface-raised disabled:opacity-40"
                          >
                            {creating === r.repoId
                              ? "Creating..."
                              : "Create index.html"}
                          </button>
                        )}
                      </div>
                    </div>
                    <p className="mt-1 text-xs text-muted">{r.message}</p>
                    <p className="mt-0.5 font-mono text-2xs text-faint">
                      {r.ready
                        ? `serves from ${r.foundAt}`
                        : `looked in ${r.checked.join(", ")}`}
                      {"  ·  "}
                      {r.path}
                    </p>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
