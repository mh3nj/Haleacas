import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../../lib/api";
import type { RepositoryFingerprint } from "../../types";
import { Badge } from "../../components/Badge";

/// The top-level "README Studio" workspace entry (spec section 16). The
/// Studio itself is inherently per-repository — it composes and writes
/// one repository's README.md — so, mirroring how the top-level Health
/// workspace links into each repository's Health tab rather than
/// aggregating health data of its own, this page is a picker that links
/// straight into a repository's README tab.
export function ReadmeLandingPage() {
  const [repos, setRepos] = useState<RepositoryFingerprint[] | null>(null);

  useEffect(() => {
    api.getRepositories().then(setRepos);
  }, []);

  if (repos === null) return null;

  if (repos.length === 0) {
    return (
      <div className="flex h-full flex-col items-start justify-center gap-2 px-8">
        <h1 className="text-lg font-semibold">No repositories indexed yet.</h1>
        <p className="max-w-md text-sm text-muted">
          Scan some repositories first, then open README Studio for one of
          them from here.
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-3">
        <h1 className="text-lg font-semibold">README Studio</h1>
        <p className="mt-1 text-sm text-muted">
          Compose a README from a repository&rsquo;s detected facts, with
          reorderable sections, a Markdown/preview toggle, and a quality
          score. Pick a repository to open it for.
        </p>
      </div>
      <div className="flex-1 overflow-y-auto">
        {repos.map((repo) => (
          <Link
            key={repo.id}
            to={`/repositories/${repo.id}?tab=readme`}
            className="flex items-center gap-4 border-b border-border px-6 py-3 hover:bg-surface-raised"
          >
            <div className="flex-1">
              <div className="text-sm font-semibold">{repo.name}</div>
              {repo.description && (
                <div className="text-2xs text-faint">{repo.description}</div>
              )}
            </div>
            {!repo.hasReadme && <Badge tone="warning">No README yet</Badge>}
          </Link>
        ))}
      </div>
    </div>
  );
}
