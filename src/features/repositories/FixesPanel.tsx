import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type {
  FixKind,
  FixOption,
  FixPreview,
  SupportedLicense,
} from "../../types";

export function FixesPanel({
  repoId,
  onApplied,
}: {
  repoId: string;
  onApplied: () => void;
}) {
  const [fixes, setFixes] = useState<FixOption[] | null>(null);
  const [licenses, setLicenses] = useState<SupportedLicense[]>([]);
  const [licenseId, setLicenseId] = useState("mit");
  const [holder, setHolder] = useState("");
  const [preview, setPreview] = useState<FixPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = () => {
    api.getAvailableFixes(repoId).then(setFixes);
  };

  useEffect(() => {
    refresh();
    api.getSupportedLicenses().then((l) => {
      setLicenses(l);
      if (l[0]) setLicenseId(l[0].id);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoId]);

  if (fixes === null) return null;

  const params = (kind: FixKind) =>
    kind === "create-license"
      ? { licenseId, copyrightHolder: holder }
      : {};

  const doPreview = async (kind: FixKind) => {
    setError(null);
    setPreview(null);
    try {
      const p = await api.previewFix(repoId, kind, params(kind));
      setPreview(p);
    } catch (e) {
      setError(String(e));
    }
  };

  const doApply = async (kind: FixKind) => {
    setBusy(true);
    setError(null);
    try {
      await api.applyFix(repoId, kind, params(kind));
      setPreview(null);
      refresh();
      onApplied();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section>
      <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
        Fixes
      </h2>
      <div className="space-y-2">
        {fixes.map((fix) => (
          <div key={fix.kind} className="border border-border px-3 py-2">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm font-medium">{fix.label}</div>
                <div className="font-mono text-2xs text-faint">
                  {fix.targetFile}
                </div>
              </div>
              {fix.available ? (
                <button
                  onClick={() => doPreview(fix.kind)}
                  className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
                >
                  Preview Fix
                </button>
              ) : (
                <span className="text-2xs text-faint">
                  {fix.unavailableReason}
                </span>
              )}
            </div>

            {fix.kind === "create-license" && fix.available && (
              <div className="mt-2 flex gap-2">
                <select
                  value={licenseId}
                  onChange={(e) => setLicenseId(e.target.value)}
                  className="rounded border border-border bg-canvas px-2 py-1 text-sm"
                >
                  {licenses.map((l) => (
                    <option key={l.id} value={l.id}>
                      {l.name}
                    </option>
                  ))}
                </select>
                <input
                  value={holder}
                  onChange={(e) => setHolder(e.target.value)}
                  placeholder="Copyright holder name"
                  className="flex-1 rounded border border-border bg-canvas px-2 py-1 text-sm"
                />
              </div>
            )}

            {preview && preview.kind === fix.kind && (
              <div className="mt-2">
                <pre className="max-h-64 overflow-y-auto border border-border bg-canvas p-2 font-mono text-2xs whitespace-pre-wrap">
                  {preview.content}
                </pre>
                <div className="mt-2 flex gap-2">
                  <button
                    onClick={() => doApply(fix.kind)}
                    disabled={busy}
                    className="rounded bg-accent px-2.5 py-1 text-sm font-medium text-canvas disabled:opacity-40"
                  >
                    Apply
                  </button>
                  <button
                    onClick={() => setPreview(null)}
                    className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}
          </div>
        ))}
        {error && <p className="text-sm text-critical">{error}</p>}
      </div>
    </section>
  );
}
