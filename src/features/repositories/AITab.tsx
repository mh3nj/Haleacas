import { useEffect, useState } from "react";
import { marked } from "marked";
import DOMPurify from "dompurify";
import { api } from "../../lib/api";
import type { AIContext, AIContextSummary, RepositoryFingerprint, Settings } from "../../types";
import { Badge } from "../../components/Badge";

interface Turn {
  question: string;
  answer: string;
  context: AIContextSummary;
}

// Each quick action pairs a pre-filled question with the context it
// needs (spec section 18's named example actions — explain repository,
// explain health issues, improve README wording, explain dependencies
// — implemented as fixed questions over the real facts/file content
// rather than separate backend endpoints; see ai/mod.rs docs).
function quickActions(hasReadme: boolean): { label: string; question: string; context: AIContext }[] {
  const actions: { label: string; question: string; context: AIContext }[] = [
    {
      label: "Explain this repository",
      question:
        "Based only on the facts above, explain what this repository appears to be and how it's organized.",
      context: { kind: "repository-facts" },
    },
    {
      label: "Explain health issues",
      question:
        "Based only on the Haleacas Health Score and Repository Doctor findings above, explain the current issues and suggest a sensible order to address them in.",
      context: { kind: "repository-facts" },
    },
    {
      label: "Explain dependencies",
      question:
        "Based only on the dependency list above, is there anything worth flagging (unusual choices, missing lockfiles, or anything that stands out)?",
      context: { kind: "repository-facts" },
    },
  ];
  if (hasReadme) {
    actions.push({
      label: "Improve README wording",
      question:
        "Based on the README.md content included above, suggest specific wording improvements. Quote the part you'd change and what you'd change it to.",
      context: { kind: "selected-files", paths: ["README.md"] },
    });
  }
  return actions;
}

function contextLabel(context: AIContext): string {
  return context.kind === "repository-facts" ? "Repository facts" : `Repository facts + ${context.paths.join(", ")}`;
}

export function AITab({ repo }: { repo: RepositoryFingerprint }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [question, setQuestion] = useState("");
  const [context, setContext] = useState<AIContext>({ kind: "repository-facts" });
  const [customPathsInput, setCustomPathsInput] = useState("");
  const [turns, setTurns] = useState<Turn[]>([]);
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedContext, setExpandedContext] = useState<number | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings);
  }, []);

  const ask = async (q: string, ctx: AIContext) => {
    const trimmed = q.trim();
    if (!trimmed) return;
    setAsking(true);
    setError(null);
    try {
      const response = await api.askAi(repo.id, ctx, trimmed);
      setTurns((prev) => [...prev, { question: trimmed, answer: response.answer, context: response.context }]);
      setQuestion("");
    } catch (e) {
      setError(String(e));
    } finally {
      setAsking(false);
    }
  };

  if (settings === null) return null;

  if (!settings.ai || settings.ai.provider === "none" || !settings.ai.endpoint.trim()) {
    return (
      <div className="max-w-2xl space-y-2">
        <h2 className="text-sm font-medium">AI isn't connected yet.</h2>
        <p className="text-sm text-muted">
          Connect a local, OpenAI-compatible provider (LM Studio, Ollama, or your own) in
          Settings to ask questions about this repository. Entirely optional; everything else in
          Haleacas works without it.
        </p>
        <a href="#/settings" className="inline-block text-sm text-accent hover:underline">
          Go to Settings →
        </a>
      </div>
    );
  }

  return (
    <div className="max-w-2xl space-y-5">
      <div className="flex flex-wrap gap-2">
        {quickActions(repo.hasReadme).map((action) => (
          <button
            key={action.label}
            onClick={() => {
              setContext(action.context);
              ask(action.question, action.context);
            }}
            disabled={asking}
            className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised disabled:opacity-40"
          >
            {action.label}
          </button>
        ))}
      </div>

      <div className="space-y-3">
        {turns.map((turn, i) => (
          <div key={i} className="border border-border">
            <div className="border-b border-border px-3 py-2">
              <div className="text-sm font-medium">{turn.question}</div>
              <button
                onClick={() => setExpandedContext(expandedContext === i ? null : i)}
                className="mt-1 text-2xs text-faint hover:text-muted"
              >
                Sent: {turn.context.description}
                {turn.context.truncated ? " (truncated)" : ""} ({expandedContext === i ? "hide" : "show"} exact text)
              </button>
              {expandedContext === i && (
                <pre className="mt-2 max-h-64 overflow-y-auto whitespace-pre-wrap rounded bg-canvas p-2 font-mono text-2xs text-muted">
                  {turn.context.text}
                </pre>
              )}
            </div>
            <div
              className="markdown-body px-3 py-2 text-sm"
              dangerouslySetInnerHTML={{
                __html: DOMPurify.sanitize(marked.parse(turn.answer, { async: false })),
              }}
            />
          </div>
        ))}
      </div>

      {asking && <p className="text-sm text-faint">Thinking… local models can take a while.</p>}
      {error && <p className="text-sm text-critical">{error}</p>}

      <div className="space-y-2 border-t border-border pt-4">
        <div className="flex items-center gap-2">
          <label className="text-2xs uppercase tracking-wide text-faint">Context:</label>
          <button
            onClick={() => setContext({ kind: "repository-facts" })}
            className={`rounded border px-2 py-0.5 text-2xs ${
              context.kind === "repository-facts" ? "border-accent text-accent" : "border-border text-muted"
            }`}
          >
            Repository facts
          </button>
          {repo.hasReadme && (
            <button
              onClick={() => setContext({ kind: "selected-files", paths: ["README.md"] })}
              className={`rounded border px-2 py-0.5 text-2xs ${
                context.kind === "selected-files" ? "border-accent text-accent" : "border-border text-muted"
              }`}
            >
              + README.md
            </button>
          )}
          <Badge>{contextLabel(context)}</Badge>
        </div>
        <div className="flex gap-2">
          <input
            value={customPathsInput}
            onChange={(e) => setCustomPathsInput(e.target.value)}
            placeholder="Extra files, repo-relative, comma-separated (e.g. src/main.rs, Cargo.toml)"
            className="flex-1 rounded border border-border bg-canvas px-2 py-1 font-mono text-xs"
          />
          <button
            onClick={() => {
              const paths = customPathsInput
                .split(",")
                .map((p) => p.trim())
                .filter((p) => p.length > 0);
              if (paths.length === 0) return;
              setContext({ kind: "selected-files", paths });
            }}
            disabled={!customPathsInput.trim()}
            className="rounded border border-border px-2.5 py-1 text-sm hover:bg-surface-raised disabled:opacity-40"
          >
            Use files
          </button>
        </div>
        <div className="flex gap-2">
          <textarea
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            placeholder="Ask a question about this repository…"
            rows={2}
            className="flex-1 rounded border border-border bg-canvas px-2 py-1.5 text-sm"
          />
          <button
            onClick={() => ask(question, context)}
            disabled={asking || !question.trim()}
            className="rounded bg-accent px-3 py-1.5 text-sm font-medium text-canvas disabled:opacity-40"
          >
            Ask
          </button>
        </div>
        <p className="text-2xs text-faint">
          Only what is shown as "Sent" above is included. Nothing else about this repo leaves
          your machine.
        </p>
      </div>
    </div>
  );
}
