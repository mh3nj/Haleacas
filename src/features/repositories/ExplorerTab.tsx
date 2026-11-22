import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type {
  DirectoryListing,
  DuplicateReport,
  ExplorerHighlights,
  FileNode,
  RepositoryFingerprint,
} from "../../types";
import { Badge } from "../../components/Badge";
import { formatSize, formatDate } from "../../lib/format";

function Breadcrumbs({
  path,
  onNavigate,
}: {
  path: string;
  onNavigate: (path: string) => void;
}) {
  const segments = path ? path.split("/") : [];
  return (
    <div className="flex flex-wrap items-center gap-1 font-mono text-2xs text-muted">
      <button onClick={() => onNavigate("")} className="hover:text-accent hover:underline">
        root
      </button>
      {segments.map((seg, i) => {
        const segPath = segments.slice(0, i + 1).join("/");
        return (
          <span key={segPath} className="flex items-center gap-1">
            <span className="text-faint">/</span>
            <button
              onClick={() => onNavigate(segPath)}
              className="hover:text-accent hover:underline"
            >
              {seg}
            </button>
          </span>
        );
      })}
    </div>
  );
}

function EntryRow({
  entry,
  onOpenDir,
  onReveal,
}: {
  entry: FileNode;
  onOpenDir: (path: string) => void;
  onReveal: (path: string) => void;
}) {
  return (
    <div className="flex items-center justify-between border-b border-border px-2 py-1.5 text-sm last:border-b-0">
      {entry.isDir ? (
        <button
          onClick={() => onOpenDir(entry.relativePath)}
          className="text-left font-medium hover:text-accent hover:underline"
        >
          {entry.name}/
        </button>
      ) : (
        <span>{entry.name}</span>
      )}
      <div className="flex items-center gap-2">
        {entry.isTest && <Badge tone="muted">test</Badge>}
        {entry.isConfig && <Badge tone="accent">config</Badge>}
        {!entry.isDir && (
          <span className="font-mono text-2xs text-faint">{formatSize(entry.sizeBytes)}</span>
        )}
        <button
          onClick={() => onReveal(entry.relativePath)}
          className="text-2xs text-faint hover:text-accent"
        >
          Reveal
        </button>
      </div>
    </div>
  );
}

function HighlightList({
  title,
  files,
  mode,
}: {
  title: string;
  files: FileNode[];
  mode: "size" | "modified";
}) {
  return (
    <div>
      <h3 className="mb-1 font-mono text-2xs uppercase tracking-wide text-faint">{title}</h3>
      {files.length === 0 ? (
        <p className="text-2xs text-faint">None found.</p>
      ) : (
        <div className="border border-border">
          {files.map((f) => (
            <div
              key={f.relativePath}
              className="flex items-center justify-between border-b border-border px-2 py-1 text-2xs last:border-b-0"
            >
              <span className="font-mono">{f.relativePath}</span>
              <span className="text-faint">
                {mode === "size" ? formatSize(f.sizeBytes) : formatDate(f.modifiedAt)}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function DuplicatesSection({ report }: { report: DuplicateReport | null }) {
  if (report === null) return null;

  const nothingFound = report.identicalGroups.length === 0 && report.similarPairs.length === 0;

  return (
    <div className="col-span-3 mt-2">
      <h3 className="mb-1 font-mono text-2xs uppercase tracking-wide text-faint">
        Duplicate &amp; similar files
      </h3>
      <p className="mb-2 text-2xs text-faint">
        {report.filesAnalyzed} file(s) analyzed. Similarity is a line-overlap score, not a
        semantic comparison. Two files can score high here while doing unrelated things if they
        share a lot of boilerplate.
        {report.truncated && " Some files/comparisons were skipped to bound scan time."}
      </p>
      {nothingFound ? (
        <p className="text-sm text-faint">No duplicates or highly similar files found.</p>
      ) : (
        <div className="space-y-3">
          {report.identicalGroups.length > 0 && (
            <div>
              <h4 className="mb-1 text-2xs text-muted">Identical files</h4>
              <div className="space-y-1">
                {report.identicalGroups.map((g) => (
                  <div key={g.files.join(",")} className="border border-border px-2 py-1.5 text-2xs">
                    <div className="flex flex-wrap gap-x-2 gap-y-1 font-mono">
                      {g.files.map((f) => (
                        <span key={f}>{f}</span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
          {report.similarPairs.length > 0 && (
            <div>
              <h4 className="mb-1 text-2xs text-muted">Similar files</h4>
              <div className="border border-border">
                {report.similarPairs.map((p) => (
                  <div
                    key={`${p.fileA}-${p.fileB}`}
                    className="flex items-center justify-between border-b border-border px-2 py-1 text-2xs last:border-b-0"
                  >
                    <span className="font-mono">
                      {p.fileA} ↔ {p.fileB}
                    </span>
                    <span className="text-faint">{p.similarity}%</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function ExplorerTab({ repo }: { repo: RepositoryFingerprint }) {
  const [path, setPath] = useState("");
  const [listing, setListing] = useState<DirectoryListing | null>(null);
  const [highlights, setHighlights] = useState<ExplorerHighlights | null>(null);
  const [duplicates, setDuplicates] = useState<DuplicateReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setError(null);
    api
      .listRepositoryDirectory(repo.id, path)
      .then(setListing)
      .catch((e) => setError(String(e)));
  }, [repo.id, path]);

  useEffect(() => {
    api.getExplorerHighlights(repo.id).then(setHighlights);
    api.getDuplicateReport(repo.id).then(setDuplicates);
  }, [repo.id]);

  const reveal = (relativePath: string) => {
    api.openInFileManager(`${repo.path}/${relativePath}`);
  };

  return (
    <div className="grid max-w-4xl grid-cols-3 gap-6">
      <div className="col-span-2">
        <Breadcrumbs path={path} onNavigate={setPath} />
        <div className="mt-2 border border-border">
          {error ? (
            <p className="px-2 py-2 text-sm text-critical">{error}</p>
          ) : listing === null ? null : listing.entries.length === 0 ? (
            <p className="px-2 py-2 text-sm text-faint">Empty directory.</p>
          ) : (
            listing.entries.map((entry) => (
              <EntryRow key={entry.relativePath} entry={entry} onOpenDir={setPath} onReveal={reveal} />
            ))
          )}
        </div>
      </div>
      <div className="space-y-4">
        {highlights === null ? null : (
          <>
            <HighlightList title="Largest files" files={highlights.largestFiles} mode="size" />
            <HighlightList
              title="Recently modified"
              files={highlights.recentlyModified}
              mode="modified"
            />
          </>
        )}
      </div>
      <DuplicatesSection report={duplicates} />
    </div>
  );
}
