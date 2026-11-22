import { useCallback, useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { RepositoryFingerprint, ScanProgress } from "../../types";
import { RepositoryRow } from "./RepositoryRow";

export function RepositoriesPage() {
  const [repos, setRepos] = useState<RepositoryFingerprint[] | null>(null);
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [skippedCount, setSkippedCount] = useState(0);
  const [hasRoots, setHasRoots] = useState<boolean | null>(null);

  const refresh = useCallback(() => {
    api.getRepositories().then(setRepos);
    api.getSettings().then((s) => setHasRoots(s.roots.length > 0));
  }, []);

  useEffect(() => {
    refresh();
    const unlistenPromise = api.onScanProgress((p) => {
      setProgress(p);
      if (p.skipped) setSkippedCount((c) => c + 1);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [refresh]);

  const addFolder = async () => {
    const picked = await api.pickRepositoryRootFolder();
    if (picked) {
      await api.addRepositoryRoot(picked);
      setHasRoots(true);
      await scan();
    }
  };

  const scan = async () => {
    setScanning(true);
    setProgress(null);
    setSkippedCount(0);
    try {
      const results = await api.scanRepositories();
      setRepos(results);
    } finally {
      setScanning(false);
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border px-6 py-3">
        <h1 className="text-lg font-semibold">Repositories</h1>
        <div className="flex gap-2">
          <button
            onClick={addFolder}
            className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
          >
            Add Folder
          </button>
          <button
            onClick={scan}
            disabled={scanning || !hasRoots}
            className="rounded bg-accent px-2.5 py-1 text-sm font-medium text-canvas disabled:opacity-40"
          >
            {scanning ? "Scanning…" : "Scan"}
          </button>
        </div>
      </div>

      {scanning && progress && (
        <div className="border-b border-border bg-surface-raised px-6 py-1.5 font-mono text-2xs text-muted">
          Indexing {progress.current} / {progress.total}
          {progress.currentPath ? `: ${progress.currentPath}` : ""}
          {skippedCount > 0 ? ` (${skippedCount} unchanged, skipped)` : ""}
        </div>
      )}

      <div className="flex-1 overflow-y-auto">
        {repos === null ? null : repos.length === 0 ? (
          <div className="flex h-full flex-col items-start justify-center gap-3 px-8">
            <h2 className="text-base font-semibold">
              {hasRoots
                ? "No repositories found in your configured folders yet."
                : "No repositories indexed yet."}
            </h2>
            <p className="max-w-md text-sm text-muted">
              {hasRoots
                ? "Run a scan, or double-check the folder actually contains Git repositories."
                : "Add a folder containing your projects and Haleacas will discover Git repositories automatically."}
            </p>
            {!hasRoots && (
              <button
                onClick={addFolder}
                className="rounded bg-accent px-3 py-1.5 text-sm font-medium text-canvas hover:bg-accent-strong"
              >
                Add Repository Folder
              </button>
            )}
          </div>
        ) : (
          repos.map((r) => <RepositoryRow key={r.id} repo={r} />)
        )}
      </div>
    </div>
  );
}
