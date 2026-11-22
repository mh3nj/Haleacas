import { useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../../lib/api";
import type { SearchReport } from "../../types";

const QUICK_QUERIES = ["TODO", "FIXME", "HACK", "deprecated", "console.log", "localhost"];

export function SearchPage() {
  const [query, setQuery] = useState("");
  const [extension, setExtension] = useState("");
  const [pathFilter, setPathFilter] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [report, setReport] = useState<SearchReport | null>(null);
  const [searching, setSearching] = useState(false);

  const runSearch = async (q: string) => {
    setQuery(q);
    if (!q.trim()) {
      setReport(null);
      return;
    }
    setSearching(true);
    try {
      const result = await api.searchRepositories(q, {
        caseSensitive,
        extension: extension.trim() || null,
        pathSubstring: pathFilter.trim() || null,
      });
      setReport(result);
    } finally {
      setSearching(false);
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-3">
        <h1 className="mb-2 text-lg font-semibold">Search</h1>
        <div className="flex gap-2">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runSearch(query)}
            placeholder="Search across all repositories…"
            className="flex-1 rounded border border-border bg-canvas px-2 py-1.5 text-sm"
          />
          <input
            value={extension}
            onChange={(e) => setExtension(e.target.value)}
            placeholder="ext"
            title="File extension filter (e.g. ts, rs, py)"
            className="w-16 rounded border border-border bg-canvas px-2 py-1.5 text-sm"
          />
          <input
            value={pathFilter}
            onChange={(e) => setPathFilter(e.target.value)}
            placeholder="path"
            title="Only matches whose file path contains this text"
            className="w-24 rounded border border-border bg-canvas px-2 py-1.5 text-sm"
          />
          <label className="flex items-center gap-1 text-sm text-muted">
            <input
              type="checkbox"
              checked={caseSensitive}
              onChange={(e) => setCaseSensitive(e.target.checked)}
            />
            Aa
          </label>
          <button
            onClick={() => runSearch(query)}
            className="rounded bg-accent px-3 py-1.5 text-sm font-medium text-canvas"
          >
            Search
          </button>
        </div>
        <div className="mt-2 flex flex-wrap gap-1.5">
          {QUICK_QUERIES.map((q) => (
            <button
              key={q}
              onClick={() => runSearch(q)}
              className="rounded border border-border px-2 py-0.5 font-mono text-2xs text-muted hover:bg-surface-raised"
            >
              {q}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        {searching ? null : report === null ? (
          <p className="px-6 py-6 text-sm text-faint">
            Search text, TODO/FIXME/HACK markers, or anything else across every indexed
            repository.
          </p>
        ) : report.matches.length === 0 ? (
          <p className="px-6 py-6 text-sm text-faint">
            No matches in {report.repositoriesSearched} repositor
            {report.repositoriesSearched === 1 ? "y" : "ies"}.
          </p>
        ) : (
          <>
            <div className="border-b border-border px-6 py-1.5 font-mono text-2xs text-muted">
              {report.matches.length} match{report.matches.length === 1 ? "" : "es"} across{" "}
              {report.repositoriesSearched} repositor
              {report.repositoriesSearched === 1 ? "y" : "ies"}
              {report.truncated && " (results truncated)"}
            </div>
            <div className="divide-y divide-border">
              {report.matches.map((m, i) => (
                <div key={`${m.repoId}-${m.file}-${m.line}-${i}`} className="px-6 py-2">
                  <div className="flex items-baseline justify-between gap-2 text-2xs">
                    <div className="flex items-baseline gap-2">
                      <Link
                        to={`/repositories/${m.repoId}`}
                        className="font-semibold text-accent hover:underline"
                      >
                        {m.repoName}
                      </Link>
                      <span className="font-mono text-faint">
                        {m.file}:{m.line}
                      </span>
                    </div>
                    <button
                      onClick={() => api.openInEditor(`${m.repoPath}/${m.file}`, m.line)}
                      className="text-faint hover:text-accent"
                    >
                      Open
                    </button>
                  </div>
                  <div className="mt-0.5 truncate font-mono text-xs">{m.lineText}</div>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
