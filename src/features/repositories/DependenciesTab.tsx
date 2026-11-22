import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { DependencyManifest, DependencyReport, RepositoryFingerprint } from "../../types";
import { Badge } from "../../components/Badge";

const kindTone: Record<string, "accent" | "muted"> = {
  direct: "accent",
  dev: "muted",
  build: "muted",
  peer: "muted",
  transitive: "muted",
};

function ManifestSection({ manifest }: { manifest: DependencyManifest }) {
  return (
    <div className="border border-border">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div>
          <span className="text-sm font-semibold">{manifest.ecosystem}</span>
          <span className="ml-2 font-mono text-2xs text-faint">{manifest.manifestFile}</span>
        </div>
        <span className="text-2xs text-faint">
          {manifest.lockfilePresent
            ? `Lockfile: ${manifest.lockfileName}`
            : "No lockfile detected"}
        </span>
      </div>
      {manifest.dependencies.length === 0 ? (
        <p className="px-3 py-2 text-sm text-faint">No dependencies declared.</p>
      ) : (
        <div className="divide-y divide-border">
          {manifest.dependencies.map((dep) => (
            <div
              key={dep.name}
              className="flex items-center justify-between px-3 py-1.5 text-sm"
            >
              <span className="font-mono">{dep.name}</span>
              <div className="flex items-center gap-2">
                <span className="font-mono text-2xs text-faint">{dep.versionDeclared}</span>
                <Badge tone={kindTone[dep.kind] ?? "muted"}>{dep.kind}</Badge>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function DependenciesTab({ repo }: { repo: RepositoryFingerprint }) {
  const [report, setReport] = useState<DependencyReport | null>(null);

  useEffect(() => {
    api.getDependencyReport(repo.id).then(setReport);
  }, [repo.id]);

  if (report === null) return null;

  if (report.manifests.length === 0) {
    return (
      <p className="max-w-2xl text-sm text-faint">
        No recognized dependency manifest was found at the repository root.
      </p>
    );
  }

  return (
    <div className="max-w-2xl space-y-4">
      <p className="text-2xs text-faint">
        Parsed from local manifests only. No network lookups, so outdated or
        freshness info is not available yet.
      </p>
      {report.manifests.map((m) => (
        <ManifestSection key={`${m.ecosystem}-${m.manifestFile}`} manifest={m} />
      ))}
    </div>
  );
}
