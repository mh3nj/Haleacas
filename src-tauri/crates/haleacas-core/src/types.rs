//! Domain types shared conceptually with the TypeScript frontend.
//!
//! Every struct here that crosses the IPC boundary derives `Serialize`/`Deserialize`
//! with `serde`'s default (camelCase-free) field names; the TS side mirrors these
//! shapes exactly in `src/types.ts`. Keep the two in sync when either changes.

use serde::{Deserialize, Serialize};

/// One item in the GitHub Inbox (spec section 21) — currently only ever
/// a CI failure, since that's the one "needs attention" signal
/// determinable without authentication (the spec's other examples,
/// "issue mentioned you" and "PR awaiting review," both need to know who
/// the authenticated user is, which unauthenticated access can't
/// provide — see STATUS.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    pub repo_id: String,
    pub repo_name: String,
    pub message: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InboxReport {
    pub items: Vec<InboxItem>,
    pub repos_checked: usize,
    pub repos_with_github: usize,
    /// True if `repos_checked < repos_with_github` — either the cap was
    /// hit or a rate-limit/error response stopped the check partway.
    pub truncated: bool,
    /// Set when the check stopped early due to an error (most likely a
    /// rate limit) rather than reaching the cap normally.
    pub error: Option<String>,
}

/// The latest published release on GitHub, if any.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub published_at: Option<String>,
    pub html_url: String,
}

/// The release *list* (spec section 22's Timeline, distinct from
/// `GitHubData.latest_release` which is only ever the single newest
/// one) — every release a page of the API returned, plus whether that
/// page was the whole story.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubReleasesReport {
    pub releases: Vec<GitHubRelease>,
    pub truncated: bool,
}

/// The most recent GitHub Actions workflow run, if any exist and are
/// visible without authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubWorkflowRun {
    pub name: String,
    /// "completed" | "in_progress" | "queued" | ... (GitHub's own status
    /// values, passed through as-is rather than remapped).
    pub status: String,
    /// "success" | "failure" | "cancelled" | ... (only present once
    /// `status` is "completed").
    pub conclusion: Option<String>,
    pub html_url: String,
    pub created_at: String,
    /// Branch the run was triggered on, when the API reports it.
    #[serde(default)]
    pub branch: Option<String>,
    /// Trigger event (push, pull_request, schedule, ...), when reported.
    #[serde(default)]
    pub event: Option<String>,
}

/// One repository inside the Actions overview, with its recent runs and
/// any per-repo fetch problem kept next to the repo it belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActionsRepoReport {
    pub repo_id: String,
    pub repo_name: String,
    pub owner: String,
    pub repo: String,
    pub html_url: String,
    pub runs: Vec<GitHubWorkflowRun>,
    pub error: Option<String>,
}

/// Cross-repo Actions overview. Built on demand from the frontend, never
/// on a timer, so the GitHub rate limit stays under control.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActionsReport {
    pub repos: Vec<ActionsRepoReport>,
    pub repos_checked: usize,
    pub repos_with_github: usize,
    pub truncated: bool,
    pub error: Option<String>,
}

/// Local GitHub Pages readiness for one repository. Purely a filesystem
/// check: does the checkout contain an entry page a static host could
/// actually serve.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PagesRepoStatus {
    pub repo_id: String,
    pub repo_name: String,
    pub path: String,
    pub ready: bool,
    /// Where the entry page was found (index.html, docs/index.html, ...),
    /// relative to the repo root. Empty when nothing was found.
    pub found_at: String,
    /// Every location that was checked, so the UI can show its work.
    pub checked: Vec<String>,
    pub has_cname: bool,
    pub has_nojekyll: bool,
    pub message: String,
}

/// Whole workspace Pages overview. Computed locally, no network needed.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PagesReport {
    pub repos: Vec<PagesRepoStatus>,
    pub total: usize,
    pub ready: usize,
    pub missing: usize,
}

/// Public GitHub repository data (spec sections 20-21). Fetched
/// unauthenticated — see the `github` module docs for what that does and
/// doesn't cover.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubData {
    pub owner: String,
    pub repo: String,
    pub html_url: String,
    pub stars: u64,
    pub forks: u64,
    /// GitHub's own `open_issues_count` — this API field counts open
    /// pull requests too (GitHub models a PR as a kind of issue
    /// internally), so it is **not** a pure issue count. Surfaced as-is
    /// with that caveat rather than silently presented as "issues only."
    pub open_issues_and_prs: u64,
    pub open_pull_requests: u64,
    /// True if there were more than one page of open PRs (capped at
    /// 100) — the count is a floor, not exact, when this is set.
    pub open_pull_requests_truncated: bool,
    pub default_branch: String,
    pub visibility: String,
    pub archived: bool,
    pub latest_release: Option<GitHubRelease>,
    pub latest_workflow_run: Option<GitHubWorkflowRun>,
}

/// A directory root the user has asked Haleacas to watch/index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryRoot {
    pub path: String,
}

/// Persisted application settings. Stored as a single JSON blob in SQLite
/// (see `db::settings`) so the schema can evolve without a migration for
/// every new preference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub roots: Vec<RepositoryRoot>,
    pub theme: String,
    pub close_behavior: CloseBehavior,
    pub launch_at_login: bool,
    /// When true, the main window starts hidden in the tray instead of
    /// showing on screen. This applies to every launch, not just login.
    /// It pairs naturally with launch at login, but works on its own too.
    #[serde(default)]
    pub start_minimized: bool,
    pub editor: Option<String>,
    /// Whether a GitHub personal access token is currently stored in the
    /// OS keychain. The token itself is never in this struct, or
    /// anywhere in the SQLite-backed settings blob this struct
    /// serializes into — only the OS keychain holds it (see
    /// `github::token`). This flag exists purely so the UI can show
    /// connection state without ever touching the secret.
    #[serde(default)]
    pub github_token_configured: bool,
    /// Local AI provider configuration (spec sections 18-19). Unlike the
    /// GitHub token, a local provider's endpoint needs no secret at all
    /// (LM Studio/Ollama serve unauthenticated on localhost), so this
    /// lives directly in the settings blob rather than the OS keychain —
    /// nothing here is sensitive.
    #[serde(default)]
    pub ai: AISettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            theme: "dark".to_string(),
            close_behavior: CloseBehavior::MinimizeToTray,
            launch_at_login: false,
            start_minimized: false,
            editor: None,
            github_token_configured: false,
            ai: AISettings::default(),
        }
    }
}

/// Which local AI provider (if any) Haleacas talks to (spec section 19).
/// `None` is the default — AI is entirely optional, and the app is fully
/// useful without it (spec section 18).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AIProviderKind {
    #[default]
    None,
    LmStudio,
    Ollama,
    Custom,
}

/// AI provider settings (spec section 19). Every provider here is a
/// local, OpenAI-compatible HTTP endpoint — LM Studio and Ollama both
/// expose one, and "Custom" covers anything else that speaks the same
/// protocol (e.g. a self-hosted server). No API key field exists on
/// purpose: spec section 19 says local providers shouldn't require one,
/// and Haleacas doesn't support cloud providers this round (see
/// `ai::client` module docs / STATUS.md).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AISettings {
    pub provider: AIProviderKind,
    pub endpoint: String,
    pub model: String,
    /// 0.0-2.0, OpenAI-API convention. `None` means "let the provider
    /// use its own default" rather than Haleacas silently picking one.
    pub temperature: Option<f32>,
}

impl Default for AISettings {
    fn default() -> Self {
        Self {
            provider: AIProviderKind::None,
            endpoint: String::new(),
            model: String::new(),
            temperature: None,
        }
    }
}

impl AISettings {
    /// True once there's enough here to actually attempt a request —
    /// a provider other than `None`, plus a non-blank endpoint. Doesn't
    /// check `model`, since Test Connection is useful even before a
    /// model is chosen (it's how the user discovers which models are
    /// available to pick from).
    pub fn enabled(&self) -> bool {
        !matches!(self.provider, AIProviderKind::None) && !self.endpoint.trim().is_empty()
    }
}

/// What to send the AI provider alongside a question (spec section 18:
/// "provide an explicit context indicator... never silently send the
/// full repository"). Only two of the spec's four example context
/// kinds — Repository facts, Selected files — are implemented this
/// round; "Selected diff" and "Full repository" are deliberately not
/// (see `ai::context` module docs / STATUS.md). `RepositoryFacts` is
/// the baseline included in *every* request regardless of variant (see
/// `ai::context::build_context`) — this enum only controls what's
/// added *on top* of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AIContext {
    RepositoryFacts,
    SelectedFiles { paths: Vec<String> },
}

/// Exactly what was sent to the AI provider for one request — the
/// "explicit context indicator" spec section 18 asks for, returned
/// alongside the answer so the frontend can show it rather than the
/// user having to trust that nothing extra went out. `text` is the
/// literal content that was placed in the prompt; showing it (behind an
/// expandable panel, not inline) is a stronger transparency guarantee
/// than a summary alone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIContextSummary {
    pub description: String,
    pub text: String,
    pub char_count: usize,
    /// True if a selected file's content had to be cut short to stay
    /// under the per-file size cap (see `explorer::read_file_for_context`).
    pub truncated: bool,
}

/// The result of one AI request (spec section 18).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIResponse {
    pub context: AIContextSummary,
    pub answer: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CloseBehavior {
    MinimizeToTray,
    Quit,
}

/// A detected technology/framework, with the evidence that justified it.
/// Detection is intentionally evidence-based (see `scanner::detect`) — we
/// never guess a framework without a concrete file/manifest to point to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechDetection {
    pub name: String,
    pub evidence: String,
}

/// Current Git status/metadata for a repository, as observed at scan time.
/// A single file's Git status, as shown in the Repository Detail page's
/// working-tree list (spec section 43).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    pub path: String,
    /// Human-readable status: "modified", "new file", "deleted", "renamed",
    /// or "conflicted" — never the raw bitflags.
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub branch: Option<String>,
    pub remote_url: Option<String>,
    pub github_slug: Option<String>,
    pub is_dirty: bool,
    pub ahead: usize,
    pub behind: usize,
    pub commit_count: usize,
    pub last_commit_hash: Option<String>,
    pub last_commit_message: Option<String>,
    pub last_commit_author: Option<String>,
    /// RFC3339 timestamp of the last commit, if any.
    pub last_commit_at: Option<String>,
    /// Per-file working-tree status. Capped (see `git_info::read`) so a
    /// repository with a huge number of changes doesn't blow up the
    /// payload — the cap is a display limit, `is_dirty` is still accurate
    /// even when the list is truncated.
    pub changed_files: Vec<GitFileChange>,
    /// True if `changed_files` was truncated by the cap.
    pub changed_files_truncated: bool,
}

/// One of the safe, one-click repository fixes from spec section 11.
/// Deliberately a closed, small set — every variant here has a real,
/// tested implementation; nothing is added to this enum speculatively.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FixKind {
    CreateGitignore,
    CreateLicense,
    CreateReadmeSkeleton,
}

/// One fix as offered to the user, with whether it's currently applicable.
/// A fix is unavailable when its target file already exists — spec section
/// 11: "never silently overwrite existing files," so an existing file
/// simply isn't offered as fixable rather than being offered and then
/// silently skipped or overwritten.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixOption {
    pub kind: FixKind,
    pub label: String,
    pub target_file: String,
    pub available: bool,
    pub unavailable_reason: Option<String>,
}

/// Inputs a fix may need. Every field is optional at this type's level;
/// each fix implementation validates that the ones *it* needs are present.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FixParams {
    /// Which license to use for `CreateLicense` — see
    /// `fixes::licenses::supported_licenses` for the closed set of ids
    /// Haleacas currently supports.
    pub license_id: Option<String>,
    pub copyright_holder: Option<String>,
}

/// What a fix would write, computed without touching disk. The UI shows
/// this before offering an Apply button.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixPreview {
    pub kind: FixKind,
    pub target_file: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedLicense {
    pub id: String,
    pub name: String,
}

/// The spec section 16 catalog — a fixed, closed set. `Custom` covers a
/// user-added section that doesn't match any of the named ones (e.g. a
/// project-specific "Deployment" or "FAQ" section); it's still fully
/// editable, just with no auto-generation behavior of its own.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReadmeSectionKind {
    Hero,
    Description,
    Features,
    Screenshots,
    Demo,
    Installation,
    QuickStart,
    Usage,
    Configuration,
    Architecture,
    Api,
    Roadmap,
    Contributing,
    License,
    Support,
    Acknowledgements,
    Custom,
}

/// One section in a README Studio document. `id` is a stable identifier
/// (independent of `kind`, since a user can add more than one `Custom`
/// section) used for reorder/edit/remove operations from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadmeSection {
    pub id: String,
    pub kind: ReadmeSectionKind,
    pub title: String,
    pub content: String,
    /// True when `content` reflects a real, confidently-detected
    /// repository fact (spec section 16: "if a command cannot be
    /// confidently detected, mark it as requiring user input"). False
    /// for a `_TODO_`-style placeholder or anything the user has since
    /// hand-edited — lets the Studio UI show which sections still need
    /// the user's attention.
    pub auto_generated: bool,
}

/// One entry in the "Add Section" catalog — every `ReadmeSectionKind`
/// paired with the title it would start with.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadmeCatalogEntry {
    pub kind: ReadmeSectionKind,
    pub default_title: String,
}

/// What `Save` would write, shown before it's allowed to happen (spec
/// section 16: "export README.md"; spec section 11's "never silently
/// overwrite" applies here too even though this isn't a Doctor fix —
/// `exists` lets the UI warn plainly when Save would replace a README
/// that's already there, rather than the write just quietly happening).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadmeSavePreview {
    pub target_file: String,
    pub content: String,
    pub exists: bool,
}

/// How a dependency relates to the project — a closed set, not a guess:
/// each variant is only ever assigned from an unambiguous manifest
/// section (e.g. npm's `devDependencies`, Cargo's `[dev-dependencies]`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyKind {
    Direct,
    Dev,
    Build,
    Peer,
    /// Declared as an indirect/transitive requirement by the manifest
    /// itself (e.g. `// indirect` in `go.mod`) — not inferred by Haleacas.
    Transitive,
}

/// One dependency entry as declared in a manifest. `version_declared` is
/// always the literal text from the manifest (a semver range, an exact
/// pin, or a marker like "(path dependency)" when no version string
/// exists) — never resolved or normalized, and never checked for
/// freshness (that needs network access Haleacas doesn't perform this
/// round; see STATUS.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub name: String,
    pub version_declared: String,
    pub kind: DependencyKind,
}

/// All dependencies parsed from one manifest file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyManifest {
    pub ecosystem: String,
    pub manifest_file: String,
    pub lockfile_present: bool,
    pub lockfile_name: Option<String>,
    pub dependencies: Vec<Dependency>,
}

/// Every dependency manifest found in a repository (spec section 13).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReport {
    pub manifests: Vec<DependencyManifest>,
}

/// One text match from a Global Search (spec section 14).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub repo_id: String,
    pub repo_name: String,
    pub repo_path: String,
    pub file: String,
    pub line: usize,
    /// The matching line, trimmed and length-capped — never the whole
    /// file, and never used to reconstruct more context than a search
    /// result needs.
    pub line_text: String,
}

/// Search options (spec section 14's filters — repository, file
/// extension, and path substring are implemented; language (beyond what
/// the extension filter already covers), modified-date, and Git status
/// are not yet, see STATUS.md).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub extension: Option<String>,
    pub repo_id: Option<String>,
    /// Keep only matches whose repo-relative file path contains this
    /// substring. Compared case-insensitively unless `case_sensitive`
    /// is set, matching the query behavior.
    pub path_substring: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchReport {
    pub matches: Vec<SearchMatch>,
    pub repositories_searched: usize,
    /// True if the total-match or per-file caps were hit — a bounded
    /// scan, not a guarantee every occurrence was found.
    pub truncated: bool,
}

/// A set of files that are byte-for-byte identical (same content hash).
/// Always has at least 2 entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub files: Vec<String>,
}

/// Two files that aren't identical but scored above the similarity
/// threshold on a purely syntactic line-overlap comparison. `similarity`
/// is a Jaccard-index percentage (0-100) over each file's set of
/// non-blank lines — explicitly *not* a claim of semantic equivalence
/// (spec section 23: "do not claim semantic equivalence unless the
/// algorithm supports it," and this one doesn't).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarPair {
    pub file_a: String,
    pub file_b: String,
    pub similarity: u8,
}

/// Local, heuristic duplicate/similar-code detection (spec section 23).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateReport {
    pub identical_groups: Vec<DuplicateGroup>,
    pub similar_pairs: Vec<SimilarPair>,
    pub files_analyzed: usize,
    /// True if a per-extension or total-comparison cap was hit, meaning
    /// not every candidate file/pair in the repository was actually
    /// checked. A bounded-cost heuristic, not an exhaustive scan.
    pub truncated: bool,
}

/// One point on the Repository Timeline (spec section 22): a commit,
/// with whatever tags point at it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    /// RFC3339 commit timestamp.
    pub date: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReport {
    pub entries: Vec<TimelineEntry>,
    /// True if the commit-count cap was hit — history older than the cap
    /// simply isn't included, not silently misrepresented as complete.
    pub truncated: bool,
}

/// Repository statistics computed from a historical commit's tree —
/// never by checking out or otherwise modifying the user's working
/// directory (spec section 22's before/after comparison). Deliberately
/// a smaller set of facts than a live scan (see `scanner::metrics`): no
/// license classification, no tech detection, no security scan at
/// arbitrary points in history — this reads a Git tree object directly,
/// not the filesystem, and duplicating that whole pipeline for tree
/// objects specifically for a "look how it changed" comparison wasn't
/// worth the added surface. `has_tests` is presence-only, not the "N
/// tests" count the spec's own example shows, since counting individual
/// tests needs language-aware parsing this doesn't do.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeStats {
    pub file_count: usize,
    pub approx_loc: usize,
    pub has_readme: bool,
    pub has_gitignore: bool,
    pub has_ci: bool,
    pub has_tests: bool,
}

/// One entry (file or directory) in a directory listing, for the
/// Repository Explorer (spec section 15).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    /// Path relative to the repository root, using forward slashes
    /// regardless of platform.
    pub relative_path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    /// RFC3339 timestamp, when available (some platforms/filesystems
    /// don't expose modification time reliably).
    pub modified_at: Option<String>,
    /// True if this looks like a test file/directory, by name/path
    /// convention (never a guess beyond a small, documented set of
    /// conventions).
    pub is_test: bool,
    /// True if this filename matches a known configuration file
    /// (package.json, Cargo.toml, etc.) — a closed, explicit list, not a
    /// heuristic on file extension alone.
    pub is_config: bool,
}

/// The contents of one directory, for lazy expansion in the UI — never
/// the whole repository tree at once.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    pub relative_path: String,
    pub entries: Vec<FileNode>,
}

/// The Explorer's "largest files" / "recently modified" panels (spec
/// section 15). Computed from a full walk, capped to the top N of each.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerHighlights {
    pub largest_files: Vec<FileNode>,
    pub recently_modified: Vec<FileNode>,
}

/// A single potential-secret match from the lightweight heuristic security
/// scanner (spec section 12). Deliberately never carries the matched text
/// itself — only where it was found and what kind of pattern matched —
/// so a real credential never ends up sitting in Haleacas' own database
/// or UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityFinding {
    pub file: String,
    pub line: usize,
    pub reason: String,
}

/// A finding the user has explicitly marked as ignored/false-positive
/// (spec section 12: "Ignore" / "Mark false positive"). Identity is
/// file + line + reason — precise enough to distinguish two different
/// findings in the same file, but not immune to drifting if the file is
/// heavily edited afterward and the same secret-shaped text moves to a
/// different line. That's a deliberate, documented trade-off for a
/// scanner that's explicitly heuristic to begin with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredFinding {
    pub file: String,
    pub line: usize,
    pub reason: String,
}

/// A single pass/fail check contributing to a category's health score
/// (spec section 9: "every score should be backed by identifiable
/// checks").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub label: String,
    pub passed: bool,
    pub detail: String,
    /// How much this check counts toward its category's score, relative
    /// to the category's other checks.
    pub weight: u32,
}

/// One health category's score plus the checks that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScore {
    pub category: String,
    pub score: u8,
    pub checks: Vec<HealthCheck>,
}

/// The "Haleacas Health Score" for a repository (spec section 9) —
/// explicitly presented as Haleacas' own scoring, not an objective
/// industry standard, and always traceable to `categories[].checks`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub overall: u8,
    pub categories: Vec<CategoryScore>,
    /// Categories the spec describes (e.g. Dependencies, Security,
    /// Maintainability) that aren't scored yet because the checks they'd
    /// need depend on later phases (dependency parsing, secret scanning).
    /// Listed explicitly rather than silently omitted or faked.
    pub not_yet_scored: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FindingSeverity {
    Critical,
    Warning,
    Improvement,
    Info,
}

/// One actionable Repository Doctor finding (spec section 10). Always
/// derived from a concrete, already-collected signal — never a guess.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorFinding {
    pub severity: FindingSeverity,
    pub title: String,
    pub explanation: String,
    pub why_it_matters: String,
    pub recommended_action: String,
    pub affected_files: Vec<String>,
}

/// The concise, automatically generated summary of a repository — its
/// "fingerprint" in the product's language (spec section 8).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryFingerprint {
    pub id: String,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub techs: Vec<TechDetection>,
    pub file_count: usize,
    /// Approximate lines of code across recognized source files. Explicitly
    /// approximate per spec section 7 — never presented as exact.
    pub approx_loc: usize,
    pub git: GitInfo,
    pub license: Option<String>,
    pub has_readme: bool,
    pub has_gitignore: bool,
    pub has_ci: bool,
    pub has_tests: bool,
    pub size_bytes: u64,
    /// RFC3339 timestamp this fingerprint was computed.
    pub scanned_at: String,
    /// The Haleacas Health Score, computed from this same fingerprint
    /// data (see `health::scoring`). Stored alongside it so the UI never
    /// needs a second round trip to show a score.
    pub health: HealthReport,
    /// Potential secrets found by the lightweight heuristic scanner
    /// (spec section 12). Capped — see `security::scan` — so a
    /// pathological match count can't blow up the payload.
    pub security_findings: Vec<SecurityFinding>,
    pub security_findings_truncated: bool,
}

/// Progress payload emitted on the `scan-progress` event while a scan runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub current: usize,
    pub total: usize,
    pub current_path: Option<String>,
    /// True when this repository's result came from the incremental-scan
    /// fast path (nothing detectably changed since the last scan) rather
    /// than a full re-analysis. Purely informational for the UI.
    pub skipped: bool,
}
