import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AIContext,
  AIResponse,
  AISettings,
  ActionsReport,
  CategoryScore,
  DependencyReport,
  DirectoryListing,
  DoctorFinding,
  DuplicateReport,
  ExplorerHighlights,
  FixKind,
  FixOption,
  FixParams,
  FixPreview,
  GitHubData,
  GitHubReleasesReport,
  IgnoredFinding,
  InboxReport,
  PagesReport,
  ReadmeCatalogEntry,
  ReadmeSavePreview,
  ReadmeSection,
  RepositoryFingerprint,
  ScanProgress,
  SearchOptions,
  SearchReport,
  SecurityFinding,
  Settings,
  SupportedLicense,
  TimelineReport,
  TreeStats,
} from "../types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),

  addRepositoryRoot: (path: string) =>
    invoke<Settings>("add_repository_root", { path }),

  removeRepositoryRoot: (path: string) =>
    invoke<Settings>("remove_repository_root", { path }),

  updateSettings: (settings: Settings) =>
    invoke<Settings>("update_settings", { settings }),

  getRepositories: () => invoke<RepositoryFingerprint[]>("get_repositories"),

  getRepository: (id: string) =>
    invoke<RepositoryFingerprint | null>("get_repository", { id }),

  getDoctorFindings: (id: string) =>
    invoke<DoctorFinding[] | null>("get_doctor_findings", { id }),

  getAvailableFixes: (id: string) => invoke<FixOption[]>("get_available_fixes", { id }),

  getSupportedLicenses: () => invoke<SupportedLicense[]>("get_supported_licenses"),

  previewFix: (id: string, kind: FixKind, params: FixParams) =>
    invoke<FixPreview>("preview_fix", { id, kind, params }),

  applyFix: (id: string, kind: FixKind, params: FixParams) =>
    invoke<RepositoryFingerprint>("apply_fix", { id, kind, params }),

  addToGitignore: (id: string, pattern: string) =>
    invoke<RepositoryFingerprint>("add_to_gitignore", { id, pattern }),

  listRepositoryDirectory: (id: string, relativePath: string) =>
    invoke<DirectoryListing>("list_repository_directory", { id, relativePath }),

  getExplorerHighlights: (id: string) =>
    invoke<ExplorerHighlights>("get_explorer_highlights", { id }),

  getDependencyReport: (id: string) =>
    invoke<DependencyReport>("get_dependency_report", { id }),

  getDuplicateReport: (id: string) =>
    invoke<DuplicateReport>("get_duplicate_report", { id }),

  searchRepositories: (query: string, options: SearchOptions) =>
    invoke<SearchReport>("search_repositories", { query, options }),

  openInEditor: (path: string, line?: number) =>
    invoke<void>("open_in_editor", { path, line: line ?? null }),

  getRepositoryTimeline: (id: string) =>
    invoke<TimelineReport>("get_repository_timeline", { id }),

  getStatsAtCommit: (id: string, commitHash: string) =>
    invoke<TreeStats>("get_stats_at_commit", { id, commitHash }),

  getGitHubData: (id: string) => invoke<GitHubData>("get_github_data", { id }),

  getRepositoryReleases: (id: string) => invoke<GitHubReleasesReport>("get_repository_releases", { id }),

  getGitHubInbox: () => invoke<InboxReport>("get_github_inbox"),

  getGitHubActions: () => invoke<ActionsReport>("get_github_actions"),

  getPagesReport: () => invoke<PagesReport>("get_pages_report"),

  createPagesIndex: (id: string) => invoke<PagesReport>("create_pages_index", { id }),

  connectGitHubToken: (token: string) => invoke<Settings>("connect_github_token", { token }),

  disconnectGitHubToken: () => invoke<Settings>("disconnect_github_token"),

  testAiConnection: (settings: AISettings) => invoke<string[]>("test_ai_connection", { settings }),

  askAi: (id: string, context: AIContext, question: string) =>
    invoke<AIResponse>("ai_ask", { id, context, question }),

  listIgnoredFindings: (id: string) => invoke<IgnoredFinding[]>("list_ignored_findings", { id }),

  ignoreFinding: (id: string, finding: SecurityFinding) =>
    invoke<RepositoryFingerprint>("ignore_finding", { id, finding }),

  unignoreFinding: (id: string, finding: IgnoredFinding) =>
    invoke<RepositoryFingerprint>("unignore_finding", { id, finding }),

  getReadmeSections: (id: string) => invoke<ReadmeSection[]>("get_readme_sections", { id }),

  getReadmeCatalog: () => invoke<ReadmeCatalogEntry[]>("get_readme_catalog"),

  renderReadmeMarkdown: (sections: ReadmeSection[]) =>
    invoke<string>("render_readme_markdown", { sections }),

  previewReadmeSave: (id: string, sections: ReadmeSection[]) =>
    invoke<ReadmeSavePreview>("preview_readme_save", { id, sections }),

  saveReadme: (id: string, sections: ReadmeSection[]) =>
    invoke<RepositoryFingerprint>("save_readme", { id, sections }),

  analyzeReadme: (id: string) => invoke<CategoryScore>("analyze_readme", { id }),

  analyzeReadmeText: (markdown: string, hasLicenseFile: boolean) =>
    invoke<CategoryScore>("analyze_readme_text", { markdown, hasLicenseFile }),

  openInFileManager: (path: string) =>
    invoke<void>("open_in_file_manager", { path }),

  openUrl: (url: string) => invoke<void>("open_url", { url }),

  pickRepositoryRootFolder: () =>
    invoke<string | null>("pick_repository_root_folder"),

  scanRepositories: () =>
    invoke<RepositoryFingerprint[]>("scan_repositories"),

  onScanProgress: (cb: (p: ScanProgress) => void): Promise<UnlistenFn> =>
    listen<ScanProgress>("scan-progress", (e) => cb(e.payload)),

  onNavigate: (cb: (route: string) => void): Promise<UnlistenFn> =>
    listen<string>("navigate", (e) => cb(e.payload)),
};
