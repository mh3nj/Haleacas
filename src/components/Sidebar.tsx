import { useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { Badge } from "./Badge";
import { NavIcon, type IconName } from "./icons";
import { useCommandCenter } from "../features/command-center/CommandCenter";

interface NavEntry {
  to: string;
  label: string;
  icon: IconName;
  shipped: boolean;
}

const entries: NavEntry[] = [
  { to: "/", label: "Overview", icon: "overview", shipped: true },
  { to: "/repositories", label: "Repositories", icon: "repos", shipped: true },
  { to: "/search", label: "Search", icon: "search", shipped: true },
  { to: "/health", label: "Health", icon: "health", shipped: true },
  { to: "/readme", label: "README Studio", icon: "readme", shipped: true },
  { to: "/github", label: "GitHub", icon: "github", shipped: true },
  { to: "/actions", label: "Actions", icon: "actions", shipped: true },
  { to: "/pages", label: "Pages", icon: "pages", shipped: true },
];

type SidebarMode = "icons" | "labels" | "full";

function useSidebarMode(): SidebarMode {
  const getMode = (): SidebarMode => {
    const w = typeof window === "undefined" ? 1200 : window.innerWidth;
    if (w < 860) return "icons";
    if (w < 1240) return "labels";
    return "full";
  };
  const [mode, setMode] = useState<SidebarMode>(getMode);

  useEffect(() => {
    const onResize = () => setMode(getMode());
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return mode;
}

export function Sidebar() {
  const { open: openCommandCenter } = useCommandCenter();
  const mode = useSidebarMode();

  const widthClass =
    mode === "icons" ? "w-14" : mode === "labels" ? "w-48" : "w-60";
  const showIcon = mode === "icons" || mode === "full";
  const showLabel = mode === "labels" || mode === "full";

  const linkBase =
    "mb-0.5 flex items-center rounded px-2 py-1.5 text-sm transition-colors";

  return (
    <nav
      className={`flex h-full shrink-0 flex-col border-r border-border bg-surface transition-all duration-150 ${widthClass}`}
      aria-label="Main navigation"
    >
      <div
        className={`flex items-center gap-2 px-3 py-3 ${
          mode === "icons" ? "justify-center px-0" : ""
        }`}
      >
        <img src="/favicon.png" alt="" className="h-4 w-4 shrink-0" />
        {showLabel && (
          <span className="truncate text-sm font-semibold tracking-tight">
            Haleacas
          </span>
        )}
      </div>

      <div className="flex-1 overflow-y-auto overflow-x-hidden px-2 py-1">
        {entries.map((entry) => (
          <NavLink
            key={entry.to}
            to={entry.to}
            end={entry.to === "/"}
            title={mode === "icons" ? entry.label : undefined}
            className={({ isActive }) =>
              `${linkBase} ${
                mode === "icons" ? "justify-center" : "justify-between gap-2"
              } ${
                isActive
                  ? "bg-surface-raised text-ink"
                  : "text-muted hover:bg-surface-raised hover:text-ink"
              }`
            }
          >
            <span
              className={`flex min-w-0 items-center gap-2 ${
                mode === "icons" ? "" : "flex-1"
              }`}
            >
              {showIcon && <NavIcon name={entry.icon} />}
              {showLabel && (
                <span className="truncate">{entry.label}</span>
              )}
            </span>
            {showLabel && !entry.shipped && <Badge>Soon</Badge>}
          </NavLink>
        ))}
        <button
          onClick={openCommandCenter}
          title={mode === "icons" ? "Command Center" : undefined}
          className={`${linkBase} w-full text-left text-muted hover:bg-surface-raised hover:text-ink ${
            mode === "icons" ? "justify-center" : "justify-between gap-2"
          }`}
        >
          <span className="flex min-w-0 items-center gap-2">
            {showIcon && <NavIcon name="command" />}
            {showLabel && <span className="truncate">Command Center</span>}
          </span>
          {mode === "full" && (
            <span className="shrink-0 font-mono text-2xs text-faint">^Space</span>
          )}
        </button>
      </div>

      <div className="border-t border-border px-2 py-2">
        <NavLink
          to="/settings"
          title={mode === "icons" ? "Settings" : undefined}
          className={({ isActive }) =>
            `${linkBase} ${
              mode === "icons" ? "justify-center" : "justify-start gap-2"
            } ${
              isActive
                ? "bg-surface-raised text-ink"
                : "text-muted hover:bg-surface-raised hover:text-ink"
            }`
          }
        >
          {showIcon && <NavIcon name="settings" />}
          {showLabel && <span className="truncate">Settings</span>}
        </NavLink>
      </div>
    </nav>
  );
}
