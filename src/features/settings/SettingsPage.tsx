import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { AIProviderKind, AISettings, Settings } from "../../types";
import { useTheme } from "../../hooks/useTheme";

const EDITOR_PRESETS = [
  { key: "vscode", label: "VS Code" },
  { key: "cursor", label: "Cursor" },
  { key: "zed", label: "Zed" },
  { key: "sublime", label: "Sublime Text" },
];

// Suggested defaults only — the user can always type a different
// endpoint. Both LM Studio and Ollama expose an OpenAI-compatible
// layer on these ports by default.
const AI_PROVIDERS: { key: AIProviderKind; label: string; defaultEndpoint: string }[] = [
  { key: "none", label: "None", defaultEndpoint: "" },
  { key: "lm-studio", label: "LM Studio", defaultEndpoint: "http://localhost:1234/v1" },
  { key: "ollama", label: "Ollama", defaultEndpoint: "http://localhost:11434/v1" },
  { key: "custom", label: "Custom", defaultEndpoint: "" },
];

export function SettingsPage() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const { theme, setTheme } = useTheme();
  const [tokenInput, setTokenInput] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [connectError, setConnectError] = useState<string | null>(null);

  const [aiDraft, setAiDraft] = useState<AISettings | null>(null);
  const [aiSaving, setAiSaving] = useState(false);
  const [aiTesting, setAiTesting] = useState(false);
  const [aiTestResult, setAiTestResult] = useState<
    { ok: true; models: string[] } | { ok: false; error: string } | null
  >(null);

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      setAiDraft(s.ai);
      // Unify theme: backend Settings.theme is now the source of truth.
      // If it disagrees with localStorage (e.g. first run after update),
      // adopt the backend value so the two can't drift apart.
      if (s.theme === "dark" || s.theme === "light") {
        setTheme(s.theme);
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const aiDraftDirty = aiDraft !== null && settings !== null &&
    JSON.stringify(aiDraft) !== JSON.stringify(settings.ai);

  const setAiProvider = (provider: AIProviderKind) => {
    if (!aiDraft) return;
    const preset = AI_PROVIDERS.find((p) => p.key === provider);
    setAiTestResult(null);
    setAiDraft({
      ...aiDraft,
      provider,
      // Only prefill the endpoint if it's currently blank — never
      // clobber something the user already typed.
      endpoint: aiDraft.endpoint.trim() === "" ? preset?.defaultEndpoint ?? "" : aiDraft.endpoint,
    });
  };

  const testAiConnection = async () => {
    if (!aiDraft) return;
    setAiTesting(true);
    setAiTestResult(null);
    try {
      const models = await api.testAiConnection(aiDraft);
      setAiTestResult({ ok: true, models });
    } catch (e) {
      setAiTestResult({ ok: false, error: String(e) });
    } finally {
      setAiTesting(false);
    }
  };

  const saveAiSettings = async () => {
    if (!aiDraft || !settings) return;
    setAiSaving(true);
    try {
      const updated = await api.updateSettings({ ...settings, ai: aiDraft });
      setSettings(updated);
      setAiDraft(updated.ai);
    } finally {
      setAiSaving(false);
    }
  };

  const addFolder = async () => {
    const picked = await api.pickRepositoryRootFolder();
    if (picked) {
      const updated = await api.addRepositoryRoot(picked);
      setSettings(updated);
    }
  };

  const removeRoot = async (path: string) => {
    const updated = await api.removeRepositoryRoot(path);
    setSettings(updated);
  };

  const setEditor = async (editor: string | null) => {
    if (!settings) return;
    const updated = await api.updateSettings({ ...settings, editor });
    setSettings(updated);
  };

  const setThemeBoth = async (next: "dark" | "light") => {
    setTheme(next);
    if (!settings) return;
    if (settings.theme !== next) {
      const updated = await api.updateSettings({ ...settings, theme: next });
      setSettings(updated);
    }
  };

  const setCloseBehavior = async (closeBehavior: Settings["closeBehavior"]) => {
    if (!settings) return;
    const updated = await api.updateSettings({ ...settings, closeBehavior });
    setSettings(updated);
  };

  const setLaunchAtLogin = async (launchAtLogin: boolean) => {
    if (!settings) return;
    const updated = await api.updateSettings({ ...settings, launchAtLogin });
    setSettings(updated);
  };

  const setStartMinimized = async (startMinimized: boolean) => {
    if (!settings) return;
    const updated = await api.updateSettings({ ...settings, startMinimized });
    setSettings(updated);
  };

  const connectGitHub = async () => {
    setConnecting(true);
    setConnectError(null);
    try {
      const updated = await api.connectGitHubToken(tokenInput);
      setSettings(updated);
      setTokenInput("");
    } catch (e) {
      setConnectError(String(e));
    } finally {
      setConnecting(false);
    }
  };

  const disconnectGitHub = async () => {
    const updated = await api.disconnectGitHubToken();
    setSettings(updated);
  };

  if (!settings) return null;

  const isKnownPreset = EDITOR_PRESETS.some((p) => p.key === settings.editor);
  const isCustom = settings.editor !== null && !isKnownPreset;

  return (
    <div className="max-w-xl px-8 py-6">
      <h1 className="mb-6 text-lg font-semibold">Settings</h1>

      <section className="mb-8">
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Appearance
        </h2>
        <div className="flex gap-2">
          <button
            onClick={() => setThemeBoth("dark")}
            className={`rounded border px-3 py-1 text-sm ${
              theme === "dark"
                ? "border-accent text-accent"
                : "border-border text-muted"
            }`}
          >
            Dark
          </button>
          <button
            onClick={() => setThemeBoth("light")}
            className={`rounded border px-3 py-1 text-sm ${
              theme === "light"
                ? "border-accent text-accent"
                : "border-border text-muted"
            }`}
          >
            Light
          </button>
        </div>
      </section>

      <section className="mb-8">
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Window
        </h2>
        <p className="mb-3 text-sm text-muted">
          What happens when you close the main window. Minimize to tray keeps Haleacas running
          in the tray; Quit exits the app. Use the tray's Quit item to exit when minimized to
          tray.
        </p>
        <div className="flex gap-2">
          <button
            onClick={() => setCloseBehavior("minimize-to-tray")}
            className={`rounded border px-3 py-1 text-sm ${
              settings.closeBehavior === "minimize-to-tray"
                ? "border-accent text-accent"
                : "border-border text-muted"
            }`}
          >
            Minimize to tray
          </button>
          <button
            onClick={() => setCloseBehavior("quit")}
            className={`rounded border px-3 py-1 text-sm ${
              settings.closeBehavior === "quit"
                ? "border-accent text-accent"
                : "border-border text-muted"
            }`}
          >
            Quit
          </button>
        </div>
        <label className="mt-3 flex items-center gap-2 text-sm text-muted">
          <input
            type="checkbox"
            checked={settings.launchAtLogin}
            onChange={(e) => setLaunchAtLogin(e.target.checked)}
          />
          Launch at login
        </label>
        <label className="mt-2 flex items-center gap-2 text-sm text-muted">
          <input
            type="checkbox"
            checked={settings.startMinimized ?? false}
            onChange={(e) => setStartMinimized(e.target.checked)}
          />
          Start minimized in the tray
        </label>
        <p className="mt-1 text-2xs text-faint">
          Launch at login registers Haleacas with your OS (Startup apps on
          Windows, Login Items on macOS, Startup Applications on Linux).
          When it starts that way it goes straight to the tray, no window
          popping up. The minimized toggle does the same for every start.
          Both apply right away and are checked again on each launch.
        </p>
      </section>

      <section className="mb-8">
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Editor
        </h2>
        <p className="mb-3 text-sm text-muted">
          Used for "Open in Editor" actions. Custom commands only get the file path, so line
          jumps are not guaranteed with editors Haleacas does not recognize.
        </p>
        <div className="flex flex-wrap gap-2">
          {EDITOR_PRESETS.map((p) => (
            <button
              key={p.key}
              onClick={() => setEditor(p.key)}
              className={`rounded border px-3 py-1 text-sm ${
                settings.editor === p.key
                  ? "border-accent text-accent"
                  : "border-border text-muted"
              }`}
            >
              {p.label}
            </button>
          ))}
          <button
            onClick={() => setEditor(isCustom ? settings.editor : "")}
            className={`rounded border px-3 py-1 text-sm ${
              isCustom ? "border-accent text-accent" : "border-border text-muted"
            }`}
          >
            Custom
          </button>
        </div>
        {isCustom && (
          <input
            value={settings.editor ?? ""}
            onChange={(e) => setEditor(e.target.value)}
            placeholder="executable name or path"
            className="mt-2 w-full rounded border border-border bg-canvas px-2 py-1 font-mono text-sm"
          />
        )}
      </section>

      <section className="mb-8">
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          GitHub
        </h2>
        <p className="mb-3 text-sm text-muted">
          Connect a personal access token for private-repository access and a much higher API
          rate limit (5,000/hour instead of 60/hour). The token lives in your OS keychain, it is
          never kept in Haleacas own database.
        </p>
        {settings.githubTokenConfigured ? (
          <div className="flex items-center gap-2">
            <span className="text-sm text-healthy">Connected</span>
            <button
              onClick={disconnectGitHub}
              className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
            >
              Disconnect
            </button>
          </div>
        ) : (
          <div className="flex gap-2">
            <input
              value={tokenInput}
              onChange={(e) => setTokenInput(e.target.value)}
              type="password"
              placeholder="ghp_…"
              className="flex-1 rounded border border-border bg-canvas px-2 py-1 font-mono text-sm"
            />
            <button
              onClick={connectGitHub}
              disabled={connecting || !tokenInput.trim()}
              className="rounded bg-accent px-2.5 py-1 text-sm font-medium text-canvas disabled:opacity-40"
            >
              {connecting ? "Connecting…" : "Connect"}
            </button>
          </div>
        )}
        {connectError && <p className="mt-2 text-sm text-critical">{connectError}</p>}
      </section>

      {aiDraft && (
        <section className="mb-8">
          <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
            AI
          </h2>
          <p className="mb-3 text-sm text-muted">
            Optional. Haleacas is fully usable without this. Connects to a local, OpenAI-compatible
            server (LM Studio, Ollama, or your own) running on your machine. No API key is used
            or needed, and nothing is sent anywhere until you explicitly ask a question in a
            repository's AI tab.
          </p>

          <div className="mb-3 flex flex-wrap gap-2">
            {AI_PROVIDERS.map((p) => (
              <button
                key={p.key}
                onClick={() => setAiProvider(p.key)}
                className={`rounded border px-3 py-1 text-sm ${
                  aiDraft.provider === p.key
                    ? "border-accent text-accent"
                    : "border-border text-muted"
                }`}
              >
                {p.label}
              </button>
            ))}
          </div>

          {aiDraft.provider !== "none" && (
            <div className="space-y-2">
              <div>
                <label className="mb-1 block text-2xs uppercase tracking-wide text-faint">
                  Endpoint
                </label>
                <input
                  value={aiDraft.endpoint}
                  onChange={(e) => {
                    setAiTestResult(null);
                    setAiDraft({ ...aiDraft, endpoint: e.target.value });
                  }}
                  placeholder="http://localhost:1234/v1"
                  className="w-full rounded border border-border bg-canvas px-2 py-1 font-mono text-sm"
                />
              </div>
              <div>
                <label className="mb-1 block text-2xs uppercase tracking-wide text-faint">
                  Model
                </label>
                <input
                  value={aiDraft.model}
                  onChange={(e) => setAiDraft({ ...aiDraft, model: e.target.value })}
                  placeholder="model id (Test Connection lists what is loaded)"
                  className="w-full rounded border border-border bg-canvas px-2 py-1 font-mono text-sm"
                />
              </div>
              <div>
                <label className="mb-1 block text-2xs uppercase tracking-wide text-faint">
                  Temperature (optional)
                </label>
                <input
                  value={aiDraft.temperature ?? ""}
                  onChange={(e) => {
                    const raw = e.target.value.trim();
                    setAiDraft({ ...aiDraft, temperature: raw === "" ? null : Number(raw) });
                  }}
                  placeholder="provider default"
                  className="w-32 rounded border border-border bg-canvas px-2 py-1 font-mono text-sm"
                />
              </div>

              <div className="flex items-center gap-2 pt-1">
                <button
                  onClick={testAiConnection}
                  disabled={aiTesting || !aiDraft.endpoint.trim()}
                  className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised disabled:opacity-40"
                >
                  {aiTesting ? "Testing…" : "Test Connection"}
                </button>
                <button
                  onClick={saveAiSettings}
                  disabled={aiSaving || !aiDraftDirty}
                  className="rounded bg-accent px-2.5 py-1 text-sm font-medium text-canvas disabled:opacity-40"
                >
                  {aiSaving ? "Saving…" : "Save"}
                </button>
              </div>

              {aiTestResult && (
                <div className="text-sm">
                  {aiTestResult.ok ? (
                    <p className="text-healthy">
                      Connected.{" "}
                      {aiTestResult.models.length === 0
                        ? "no models currently loaded."
                        : `${aiTestResult.models.length} model(s) available: ${aiTestResult.models.join(", ")}`}
                    </p>
                  ) : (
                    <p className="text-critical">{aiTestResult.error}</p>
                  )}
                </div>
              )}
            </div>
          )}
        </section>
      )}

      <section>
        <h2 className="mb-2 font-mono text-2xs uppercase tracking-wide text-faint">
          Repository roots
        </h2>
        <p className="mb-3 text-sm text-muted">
          Folders Haleacas scans recursively for Git repositories.
        </p>

        <div className="mb-3 divide-y divide-border rounded border border-border">
          {settings.roots.length === 0 ? (
            <div className="px-3 py-2 text-sm text-faint">
              No folders configured yet.
            </div>
          ) : (
            settings.roots.map((r) => (
              <div
                key={r.path}
                className="flex items-center justify-between px-3 py-2"
              >
                <span className="font-mono text-xs">{r.path}</span>
                <button
                  onClick={() => removeRoot(r.path)}
                  className="text-xs text-muted hover:text-critical"
                >
                  Remove
                </button>
              </div>
            ))
          )}
        </div>

        <button
          onClick={addFolder}
          className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised"
        >
          Add Folder
        </button>
      </section>
    </div>
  );
}
