import { useEffect } from "react";
import { HashRouter, Route, Routes, useNavigate } from "react-router-dom";
import { Sidebar } from "./components/Sidebar";
import { OverviewPage } from "./features/overview/OverviewPage";
import { RepositoriesPage } from "./features/repositories/RepositoriesPage";
import { RepositoryDetailPage } from "./features/repositories/RepositoryDetailPage";
import { SettingsPage } from "./features/settings/SettingsPage";
import { HealthPage } from "./features/health/HealthPage";
import { SearchPage } from "./features/search/SearchPage";
import { CommandCenterProvider } from "./features/command-center/CommandCenter";
import { GitHubInboxPage } from "./features/github/GitHubInboxPage";
import { ActionsPage } from "./features/actions/ActionsPage";
import { PagesPage } from "./features/pages/PagesPage";
import { ReadmeLandingPage } from "./features/readme/ReadmeLandingPage";
import { api } from "./lib/api";
import { useTheme } from "./hooks/useTheme";

function TrayNavigationListener() {
  const navigate = useNavigate();

  useEffect(() => {
    const unlistenPromise = api.onNavigate((route) => navigate(route));
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [navigate]);

  return null;
}

function Layout() {
  return (
    <div className="flex h-screen">
      <Sidebar />
      <main className="min-w-0 flex-1 overflow-y-auto bg-canvas">
        <Routes>
          <Route path="/" element={<OverviewPage />} />
          <Route path="/repositories" element={<RepositoriesPage />} />
          <Route path="/repositories/:id" element={<RepositoryDetailPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/health" element={<HealthPage />} />
          <Route path="/readme" element={<ReadmeLandingPage />} />
          <Route
            path="/github"
            element={<GitHubInboxPage />}
          />
          <Route path="/actions" element={<ActionsPage />} />
          <Route path="/pages" element={<PagesPage />} />
        </Routes>
      </main>
    </div>
  );
}

export default function App() {
  useTheme();

  return (
    <HashRouter>
      <CommandCenterProvider>
        <TrayNavigationListener />
        <Layout />
      </CommandCenterProvider>
    </HashRouter>
  );
}
