import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { IgnoredFinding, RepositoryFingerprint, SecurityFinding } from "../../types";

function findingKey(f: { file: string; line: number; reason: string }): string {
  return `${f.file}:${f.line}:${f.reason}`;
}

export function SecurityFindingsPanel({
  repo,
  onChanged,
}: {
  repo: RepositoryFingerprint;
  onChanged: () => void;
}) {
  const [ignored, setIgnored] = useState<IgnoredFinding[] | null>(null);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const refreshIgnored = () => {
    api.listIgnoredFindings(repo.id).then(setIgnored);
  };

  useEffect(() => {
    refreshIgnored();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repo.id]);

  const ignore = async (finding: SecurityFinding) => {
    setBusyKey(findingKey(finding));
    try {
      await api.ignoreFinding(repo.id, finding);
      refreshIgnored();
      onChanged();
    } finally {
      setBusyKey(null);
    }
  };

  const unignore = async (finding: IgnoredFinding) => {
    setBusyKey(findingKey(finding));
    try {
      await api.unignoreFinding(repo.id, finding);
      refreshIgnored();
      onChanged();
    } finally {
      setBusyKey(null);
    }
  };

  const addToGitignore = async (finding: SecurityFinding) => {
    const pattern = window.prompt(
      "Add a pattern to .gitignore for this finding (repo-relative, e.g. the file path):",
      finding.file
    );
    if (pattern === null) return;
    setBusyKey(findingKey(finding));
    setActionError(null);
    try {
      await api.addToGitignore(repo.id, pattern);
      onChanged();
    } catch (e) {
      setActionError(String(e));
    } finally {
      setBusyKey(null);
    }
  };

  if (repo.securityFindings.length === 0 && (!ignored || ignored.length === 0)) {
    return null;
  }

  return (
    <section>
      <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
        Security findings (heuristic scanner)
      </h2>
      {repo.securityFindings.length === 0 ? (
        <p className="text-sm text-faint">No active findings.</p>
      ) : (
        <div className="border border-border">
          {repo.securityFindings.map((f) => (
            <div
              key={findingKey(f)}
              className="flex items-center justify-between border-b border-border px-3 py-1.5 text-sm last:border-b-0"
            >
              <span className="font-mono text-xs">
                {f.file}:{f.line} ({f.reason})
              </span>
              <span className="flex gap-3">
                <button
                  onClick={() => addToGitignore(f)}
                  disabled={busyKey === findingKey(f)}
                  title="Append this file's pattern to .gitignore"
                  className="text-2xs text-faint hover:text-accent disabled:opacity-40"
                >
                  Gitignore
                </button>
                <button
                  onClick={() => ignore(f)}
                  disabled={busyKey === findingKey(f)}
                  className="text-2xs text-faint hover:text-accent disabled:opacity-40"
                >
                  Ignore
                </button>
              </span>
            </div>
          ))}
        </div>
      )}
      {repo.securityFindingsTruncated && (
        <p className="mt-1 text-2xs text-faint">
          Scan stopped early after the finding cap. There may be more.
        </p>
      )}
      {actionError && <p className="mt-2 text-sm text-critical">{actionError}</p>}

      {ignored && ignored.length > 0 && (
        <div className="mt-3">
          <h3 className="mb-1 text-2xs text-muted">Ignored / marked false positive</h3>
          <div className="border border-border">
            {ignored.map((f) => (
              <div
                key={findingKey(f)}
                className="flex items-center justify-between border-b border-border px-3 py-1.5 text-sm last:border-b-0"
              >
                <span className="font-mono text-xs text-faint">
                  {f.file}:{f.line} ({f.reason})
                </span>
                <button
                  onClick={() => unignore(f)}
                  disabled={busyKey === findingKey(f)}
                  className="text-2xs text-faint hover:text-accent disabled:opacity-40"
                >
                  Un-ignore
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
