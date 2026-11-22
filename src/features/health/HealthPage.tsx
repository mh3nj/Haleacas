import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../../lib/api";
import type { RepositoryFingerprint } from "../../types";
import { HealthScore } from "../../components/HealthScore";

export function HealthPage() {
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
          Scan some repositories first and their health scores will show up
          here, worst first.
        </p>
      </div>
    );
  }

  const sorted = [...repos].sort((a, b) => a.health.overall - b.health.overall);

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-3">
        <h1 className="text-lg font-semibold">Repository Health</h1>
        <p className="mt-1 text-sm text-muted">
          The Haleacas Health Score is Haleacas own scoring based on checks
          it can verify (documentation, structure, testing, Git hygiene,
          configuration). It is not an industry-standard metric. Dependency
          and security scoring are not built yet, so they never factor in.
        </p>
      </div>
      <div className="flex-1 overflow-y-auto">
        {sorted.map((repo) => (
          <Link
            key={repo.id}
            to={`/repositories/${repo.id}`}
            className="flex items-center gap-4 border-b border-border px-6 py-3 hover:bg-surface-raised"
          >
            <HealthScore value={repo.health.overall} />
            <div className="flex-1">
              <div className="text-sm font-semibold">{repo.name}</div>
              <div className="flex gap-3 font-mono text-2xs text-faint">
                {repo.health.categories.map((c) => (
                  <span key={c.category}>
                    {c.category} {c.score}
                  </span>
                ))}
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
