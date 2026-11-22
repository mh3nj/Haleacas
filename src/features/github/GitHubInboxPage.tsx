import { useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../../lib/api";
import type { InboxReport } from "../../types";

export function GitHubInboxPage() {
  const [report, setReport] = useState<InboxReport | null>(null);
  const [checking, setChecking] = useState(false);

  const check = async () => {
    setChecking(true);
    try {
      const result = await api.getGitHubInbox();
      setReport(result);
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-3">
        <h1 className="text-lg font-semibold">GitHub Activity</h1>
        <p className="mt-1 text-sm text-muted">
          Checks the most recent CI run for each repository with a detected GitHub remote. It
          only runs when you ask, since each check spends part of the GitHub request
          budget (60/hour without a token, 5,000/hour with a token connected in Settings), so
          it stays capped and never runs on its own.
        </p>
        <button
          onClick={check}
          disabled={checking}
          className="mt-3 rounded bg-accent px-3 py-1.5 text-sm font-medium text-canvas disabled:opacity-40"
        >
          {checking ? "Checking…" : "Check GitHub Activity"}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {report === null ? (
          <p className="px-6 py-6 text-sm text-faint">
            No check has been run yet this session.
          </p>
        ) : report.reposWithGithub === 0 ? (
          <p className="px-6 py-6 text-sm text-faint">
            No indexed repositories have a detected GitHub remote yet.
          </p>
        ) : (
          <>
            <div className="border-b border-border px-6 py-1.5 font-mono text-2xs text-muted">
              Checked {report.reposChecked} of {report.reposWithGithub} repositories
              {report.truncated && !report.error ? " (capped)" : ""}
            </div>
            {report.error && (
              <p className="border-b border-border px-6 py-2 text-sm text-critical">
                Stopped early: {report.error}
              </p>
            )}
            {report.items.length === 0 ? (
              <p className="px-6 py-6 text-sm text-faint">
                Nothing needs attention in the repositories checked.
              </p>
            ) : (
              <div className="divide-y divide-border">
                {report.items.map((item) => (
                  <div key={item.repoId} className="flex items-center justify-between px-6 py-2">
                    <div>
                      <Link
                        to={`/repositories/${item.repoId}`}
                        className="text-sm font-semibold text-accent hover:underline"
                      >
                        {item.repoName}
                      </Link>
                      <div className="text-sm text-muted">{item.message}</div>
                    </div>
                    <button
                      onClick={() => api.openUrl(item.htmlUrl)}
                      className="text-2xs text-faint hover:text-accent"
                    >
                      Open
                    </button>
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
