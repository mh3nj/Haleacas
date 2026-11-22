import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate } from "react-router-dom";
import { api } from "../../lib/api";
import type { RepositoryFingerprint } from "../../types";

interface Command {
  id: string;
  label: string;
  hint?: string;
  run: () => void | Promise<void>;
}

const CommandCenterContext = createContext<{ open: () => void } | null>(null);

export function useCommandCenter() {
  const ctx = useContext(CommandCenterContext);
  if (!ctx) throw new Error("useCommandCenter must be used within CommandCenterProvider");
  return ctx;
}

const STATIC_DESTINATIONS: { label: string; path: string }[] = [
  { label: "Open Overview", path: "/" },
  { label: "Open Repositories", path: "/repositories" },
  { label: "Open Search", path: "/search" },
  { label: "Open Health", path: "/health" },
  { label: "Open README Studio", path: "/readme" },
  { label: "Open GitHub Activity", path: "/github" },
  { label: "Open GitHub Actions", path: "/actions" },
  { label: "Open GitHub Pages", path: "/pages" },
  { label: "Open Settings", path: "/settings" },
];

export function CommandCenterProvider({ children }: { children: React.ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [repos, setRepos] = useState<RepositoryFingerprint[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();

  const open = useCallback(() => {
    api.getRepositories().then(setRepos);
    setQuery("");
    setSelected(0);
    setIsOpen(true);
  }, []);
  const close = useCallback(() => setIsOpen(false), []);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // In-window shortcut only — not an OS-global hotkey (that needs a
      // separate plugin/permission and platform testing this build
      // doesn't have). Works whenever the Haleacas window has focus.
      if ((e.ctrlKey || e.metaKey) && e.code === "Space") {
        e.preventDefault();
        setIsOpen((prev) => {
          if (!prev) {
            api.getRepositories().then(setRepos);
            setQuery("");
            setSelected(0);
          }
          return !prev;
        });
      }
      if (e.key === "Escape") {
        setIsOpen(false);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (isOpen) {
      // Focus after the overlay actually renders.
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [isOpen]);

  const commands = useMemo<Command[]>(() => {
    const nav: Command[] = STATIC_DESTINATIONS.map((d) => ({
      id: `nav-${d.path}`,
      label: d.label,
      run: () => navigate(d.path),
    }));

    const scan: Command[] = [
      {
        id: "scan-repositories",
        label: "Scan Repositories",
        run: async () => {
          navigate("/repositories");
          await api.scanRepositories();
        },
      },
    ];

    const perRepo: Command[] = repos.flatMap((repo) => {
      const cmds: Command[] = [
        {
          id: `open-${repo.id}`,
          label: `Open ${repo.name}`,
          run: () => navigate(`/repositories/${repo.id}`),
        },
        {
          id: `editor-${repo.id}`,
          label: `Open ${repo.name} in Editor`,
          run: () => api.openInEditor(repo.path),
        },
        {
          id: `folder-${repo.id}`,
          label: `Open ${repo.name} Folder`,
          run: () => api.openInFileManager(repo.path),
        },
        {
          id: `health-${repo.id}`,
          label: `Health ${repo.name}`,
          hint: `${repo.health.overall}/100`,
          run: () => navigate(`/repositories/${repo.id}?tab=health`),
        },
        {
          id: `readme-${repo.id}`,
          label: `README ${repo.name}`,
          run: () => navigate(`/repositories/${repo.id}?tab=readme`),
        },
        {
          id: `timeline-${repo.id}`,
          label: `Timeline ${repo.name}`,
          run: () => navigate(`/repositories/${repo.id}?tab=timeline`),
        },
        {
          id: `ai-${repo.id}`,
          label: `Ask AI about ${repo.name}`,
          run: () => navigate(`/repositories/${repo.id}?tab=ai`),
        },
      ];
      if (repo.git.githubSlug) {
        cmds.push({
          id: `github-${repo.id}`,
          label: `GitHub ${repo.name}`,
          run: () => api.openUrl(`https://github.com/${repo.git.githubSlug}`),
        });
      }
      return cmds;
    });

    return [...nav, ...scan, ...perRepo];
  }, [repos, navigate]);

  const filtered = useMemo(() => {
    if (!query.trim()) return commands.slice(0, 8);
    const q = query.toLowerCase();
    return commands.filter((c) => c.label.toLowerCase().includes(q)).slice(0, 20);
  }, [commands, query]);

  const runSelected = () => {
    const cmd = filtered[selected];
    if (cmd) {
      cmd.run();
      close();
    }
  };

  return (
    <CommandCenterContext.Provider value={{ open }}>
      {children}
      {isOpen && (
        <div
          className="fixed inset-0 z-50 flex items-start justify-center bg-black/40 pt-32"
          onClick={close}
        >
          <div
            className="w-full max-w-lg border border-border-strong bg-surface shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <input
              ref={inputRef}
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setSelected(0);
              }}
              onKeyDown={(e) => {
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setSelected((s) => Math.min(s + 1, filtered.length - 1));
                } else if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setSelected((s) => Math.max(s - 1, 0));
                } else if (e.key === "Enter") {
                  e.preventDefault();
                  runSelected();
                }
              }}
              placeholder="Type a command…"
              className="w-full border-b border-border bg-transparent px-3 py-2.5 text-sm outline-none"
            />
            <div className="max-h-80 overflow-y-auto">
              {filtered.length === 0 ? (
                <p className="px-3 py-3 text-sm text-faint">No matching commands.</p>
              ) : (
                filtered.map((cmd, i) => (
                  <div
                    key={cmd.id}
                    onMouseEnter={() => setSelected(i)}
                    onClick={() => {
                      cmd.run();
                      close();
                    }}
                    className={`flex cursor-pointer items-center justify-between px-3 py-1.5 text-sm ${
                      i === selected ? "bg-surface-raised text-ink" : "text-muted"
                    }`}
                  >
                    <span>{cmd.label}</span>
                    {cmd.hint && (
                      <span className="font-mono text-2xs text-faint">{cmd.hint}</span>
                    )}
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </CommandCenterContext.Provider>
  );
}
