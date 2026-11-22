//! GitHub integration (spec sections 20-21).
//!
//! Supports both unauthenticated access (public repositories only,
//! subject to GitHub's 60 requests/hour per-IP limit) and, when a
//! personal access token is configured (stored via the OS keychain, see
//! `token` submodule), authenticated access — private repositories and
//! a 5,000 requests/hour limit. Every function that talks to GitHub
//! takes `token: Option<&str>`; `None` means unauthenticated.
//!
//! Every network call here is a plain `GET` to a read-only endpoint —
//! nothing here can modify anything on GitHub, authenticated or not.
//!
//! Split deliberately into pure parsing functions (unit-tested against
//! real fixture JSON shaped exactly like GitHub's actual API responses)
//! and thin HTTP-calling wrappers (covered by `#[ignore]`d tests that hit
//! the live API — not run by default `cargo test`, since a real network
//! call has no place in a normal test run, but run manually to confirm
//! this actually works against the real GitHub API rather than only a
//! fixture I wrote by hand).

pub mod token;

use crate::types::{GitHubData, GitHubRelease, GitHubReleasesReport, GitHubWorkflowRun, InboxItem};
use serde::Deserialize;

const USER_AGENT: &str = "haleacas-repository-intelligence-toolkit";

/// Builds the headers every GitHub request needs — GitHub's expected
/// `Accept` header, plus `Authorization: Bearer <token>` when a token is
/// supplied. Pure and separate from `build_client` specifically so it's
/// directly inspectable in tests (a built `reqwest::Client` doesn't
/// expose its default headers for introspection afterward).
fn build_headers(token: Option<&str>) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
    );
    if let Some(t) = token {
        let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {t}"))
            .map_err(|_| "GitHub token contains characters that can't go in a header.".to_string())?;
        headers.insert(reqwest::header::AUTHORIZATION, value);
    }
    Ok(headers)
}

/// Builds a client with GitHub's expected `Accept` header, and — when a
/// token is supplied — an `Authorization: Bearer <token>` default header
/// so every request made with this client is authenticated. Never logs
/// or otherwise surfaces the token itself.
fn build_client(token: Option<&str>) -> Result<reqwest::blocking::Client, String> {
    let headers = build_headers(token)?;

    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()
        .map_err(|e| e.to_string())
}

fn rate_limit_message(authenticated: bool) -> String {
    if authenticated {
        "GitHub API rate limit reached (5,000 requests/hour for authenticated access)."
            .to_string()
    } else {
        "GitHub API rate limit reached (60 requests/hour for unauthenticated access). Connect a \
         GitHub token in Settings for a much higher limit."
            .to_string()
    }
}

/// Cap on how many repositories a single Inbox check queries, since each
/// one costs a real request against the unauthenticated 60/hour budget.
/// Leaves headroom for other GitHub-tab views in the same hour rather
/// than spending the whole budget on one Inbox check.
pub const INBOX_REPO_CAP: usize = 15;

/// The same cap, but for when a token is configured — 5,000/hour instead
/// of 60/hour has much more room, though still capped rather than
/// unbounded so one Inbox check can't spend the whole hour's budget on
/// its own.
pub const INBOX_REPO_CAP_AUTHENTICATED: usize = 200;

/// One candidate repository for the Inbox check — just enough identity
/// to make the request and label the result.
pub struct InboxCandidate {
    pub repo_id: String,
    pub repo_name: String,
    pub owner: String,
    pub repo: String,
}

/// Pure: given a candidate and its already-fetched CI status, decide
/// whether it belongs in the Inbox. Only a `completed` run with a
/// `failure` conclusion counts — `in_progress`/`queued` runs and
/// non-failure conclusions (`success`, `cancelled`, `skipped`, etc.)
/// don't need attention.
pub fn build_inbox_item(candidate: &InboxCandidate, run: &GitHubWorkflowRun) -> Option<InboxItem> {
    if run.status == "completed" && run.conclusion.as_deref() == Some("failure") {
        Some(InboxItem {
            repo_id: candidate.repo_id.clone(),
            repo_name: candidate.repo_name.clone(),
            message: format!("CI failed: {}", run.name),
            html_url: run.html_url.clone(),
        })
    } else {
        None
    }
}

#[derive(Deserialize)]
struct RepoResponse {
    html_url: String,
    stargazers_count: u64,
    forks_count: u64,
    open_issues_count: u64,
    default_branch: String,
    visibility: Option<String>,
    #[serde(default)]
    archived: bool,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    html_url: String,
}

fn map_release(r: ReleaseResponse) -> GitHubRelease {
    GitHubRelease {
        tag_name: r.tag_name,
        name: r.name,
        published_at: r.published_at,
        html_url: r.html_url,
    }
}

#[derive(Deserialize)]
struct WorkflowRunsResponse {
    workflow_runs: Vec<WorkflowRunResponse>,
}

#[derive(Deserialize)]
struct WorkflowRunResponse {
    name: Option<String>,
    status: String,
    conclusion: Option<String>,
    html_url: String,
    created_at: String,
    #[serde(default)]
    head_branch: Option<String>,
    #[serde(default)]
    event: Option<String>,
}

fn parse_repo(json: &str) -> Result<RepoResponse, String> {
    serde_json::from_str(json).map_err(|e| format!("Unexpected repository response: {e}"))
}

fn parse_release(json: &str) -> Result<ReleaseResponse, String> {
    serde_json::from_str(json).map_err(|e| format!("Unexpected release response: {e}"))
}

fn parse_releases(json: &str) -> Result<Vec<ReleaseResponse>, String> {
    serde_json::from_str(json).map_err(|e| format!("Unexpected releases response: {e}"))
}

fn parse_workflow_runs(json: &str) -> Result<WorkflowRunsResponse, String> {
    serde_json::from_str(json).map_err(|e| format!("Unexpected workflow runs response: {e}"))
}

/// Assembles the full `GitHubData` from each endpoint's already-parsed
/// response. Pure — no network access — so it's directly unit-testable.
fn map_workflow_run(w: WorkflowRunResponse) -> GitHubWorkflowRun {
    GitHubWorkflowRun {
        name: w.name.unwrap_or_else(|| "(unnamed workflow)".to_string()),
        status: w.status,
        conclusion: w.conclusion,
        html_url: w.html_url,
        created_at: w.created_at,
        branch: w.head_branch,
        event: w.event,
    }
}

fn assemble(
    owner: &str,
    repo: &str,
    repo_response: RepoResponse,
    release: Option<ReleaseResponse>,
    open_pr_count: u64,
    open_pr_truncated: bool,
    workflow_run: Option<WorkflowRunResponse>,
) -> GitHubData {
    GitHubData {
        owner: owner.to_string(),
        repo: repo.to_string(),
        html_url: repo_response.html_url,
        stars: repo_response.stargazers_count,
        forks: repo_response.forks_count,
        open_issues_and_prs: repo_response.open_issues_count,
        open_pull_requests: open_pr_count,
        open_pull_requests_truncated: open_pr_truncated,
        default_branch: repo_response.default_branch,
        visibility: repo_response.visibility.unwrap_or_else(|| "public".to_string()),
        archived: repo_response.archived,
        latest_release: release.map(map_release),
        latest_workflow_run: workflow_run.map(map_workflow_run),
    }
}

/// Fetch data for `owner/repo`. Unauthenticated when `token` is `None`
/// (public repos only, 60 req/hr per IP); authenticated when `Some`
/// (private repos visible too, 5,000 req/hr). Rate-limit and auth errors
/// are surfaced with an explanation, not a raw status code.
pub fn fetch(owner: &str, repo: &str, token: Option<&str>) -> Result<GitHubData, String> {
    let client = build_client(token)?;
    let authenticated = token.is_some();

    let repo_response = fetch_repo(&client, owner, repo, authenticated)?;
    let release = fetch_latest_release(&client, owner, repo)?;
    let (open_pr_count, open_pr_truncated) = fetch_open_pr_count(&client, owner, repo)?;
    let workflow_run = fetch_latest_workflow_run(&client, owner, repo)?;

    Ok(assemble(
        owner,
        repo,
        repo_response,
        release,
        open_pr_count,
        open_pr_truncated,
        workflow_run,
    ))
}

/// A single-endpoint fetch of the release *list* — spec section 22
/// (Repository Timeline): "Use: commits, tags, releases... where
/// possible, allow comparing." `fetch` above only ever surfaces the
/// single latest release; this is what lets the Timeline show which of
/// a repository's *tags* also have a GitHub Release behind them, not
/// just the newest one.
///
/// Capped to one page (`per_page=RELEASES_PAGE_SIZE`) — same
/// `truncated`-via-Link-header pattern as `fetch_open_pr_count`, so a
/// repository with more releases than that isn't silently reported as
/// complete.
const RELEASES_PAGE_SIZE: u32 = 50;

pub fn fetch_releases(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<GitHubReleasesReport, String> {
    let client = build_client(token)?;
    let authenticated = token.is_some();

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases?per_page={RELEASES_PAGE_SIZE}"
    );
    let response = get(&client, &url)?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(
            "Repository not found on GitHub. It may be private, renamed, or deleted."
                .to_string(),
        );
    }
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("GitHub rejected the configured token. It may be invalid or expired.".to_string());
    }
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(rate_limit_message(authenticated));
    }
    if !response.status().is_success() {
        return Err(format!("GitHub returned an unexpected status: {}", response.status()));
    }

    let truncated = response
        .headers()
        .get("link")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("rel=\"next\""))
        .unwrap_or(false);

    let body = response.text().map_err(|e| e.to_string())?;
    let releases = parse_releases(&body)?.into_iter().map(map_release).collect();

    Ok(GitHubReleasesReport { releases, truncated })
}

/// A single-endpoint, single-request fetch of just the latest CI run —
/// deliberately the cheapest possible GitHub check (1 request instead of
/// `fetch`'s 4), because this is the one used repeatedly across many
/// repositories by the GitHub Inbox (spec section 21), where the
/// unauthenticated 60/hour rate limit is the binding constraint.
///
/// Unlike the internal `fetch_latest_workflow_run` helper (used by the
/// full `fetch`, where a failure here shouldn't sink the whole repo
/// summary over one optional field), this function does **not** treat a
/// non-success response as "no runs" — a 403 rate-limit response and "no
/// CI configured" are very different facts, and the Inbox needs to tell
/// them apart: silently reporting "no CI issues" when the real answer is
/// "couldn't check" would be exactly the kind of false claim the product
/// is supposed to avoid.
pub fn fetch_ci_status(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Option<GitHubWorkflowRun>, String> {
    let client = build_client(token)?;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/actions/runs?per_page=1");
    let response = get(&client, &url)?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("GitHub rejected the configured token. It may be invalid or expired.".to_string());
    }
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(rate_limit_message(token.is_some()));
    }
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(
            "Repository not found on GitHub. It may be private, renamed, or deleted."
                .to_string(),
        );
    }
    if !response.status().is_success() {
        return Err(format!("GitHub returned an unexpected status: {}", response.status()));
    }

    let body = response.text().map_err(|e| e.to_string())?;
    let runs = parse_workflow_runs(&body)?;
    Ok(runs.workflow_runs.into_iter().next().map(map_workflow_run))
}

/// Recent workflow runs for one repo, newest first. Used by the Actions
/// overview page. Same error shape as fetch_ci_status: auth and rate
/// limit problems are real errors, not empty lists.
pub const ACTIONS_RUNS_PER_REPO: usize = 10;

pub fn fetch_workflow_runs(
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<GitHubWorkflowRun>, String> {
    let client = build_client(token)?;

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/actions/runs?per_page={}",
        ACTIONS_RUNS_PER_REPO
    );
    let response = get(&client, &url)?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("GitHub rejected the configured token. It may be invalid or expired.".to_string());
    }
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(rate_limit_message(token.is_some()));
    }
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(
            "Repository not found on GitHub. It may be private, renamed, or deleted."
                .to_string(),
        );
    }
    if !response.status().is_success() {
        return Err(format!("GitHub returned an unexpected status: {}", response.status()));
    }

    let body = response.text().map_err(|e| e.to_string())?;
    let runs = parse_workflow_runs(&body)?;
    Ok(runs.workflow_runs.into_iter().map(map_workflow_run).collect())
}

fn get(client: &reqwest::blocking::Client, url: &str) -> Result<reqwest::blocking::Response, String> {
    client.get(url).send().map_err(|e| {
        if e.is_timeout() {
            "GitHub request timed out.".to_string()
        } else {
            format!("Could not reach GitHub: {e}")
        }
    })
}

fn fetch_repo(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
    authenticated: bool,
) -> Result<RepoResponse, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let response = get(client, &url)?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(
            "Repository not found on GitHub. It may be private, renamed, or deleted."
                .to_string(),
        );
    }
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("GitHub rejected the configured token. It may be invalid or expired.".to_string());
    }
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(rate_limit_message(authenticated));
    }
    if !response.status().is_success() {
        return Err(format!("GitHub returned an unexpected status: {}", response.status()));
    }

    let body = response.text().map_err(|e| e.to_string())?;
    parse_repo(&body)
}

fn fetch_latest_release(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
) -> Result<Option<ReleaseResponse>, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let response = get(client, &url)?;

    // No releases at all is a normal, common case — not an error.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Ok(None); // don't fail the whole fetch over an optional field
    }

    let body = response.text().map_err(|e| e.to_string())?;
    Ok(parse_release(&body).ok())
}

fn fetch_open_pr_count(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
) -> Result<(u64, bool), String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls?state=open&per_page=100");
    let response = get(client, &url)?;
    if !response.status().is_success() {
        return Ok((0, false));
    }

    // A Link header with rel="next" means there's more than one page —
    // we only fetch the first, so report the count as a floor.
    let truncated = response
        .headers()
        .get("link")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("rel=\"next\""))
        .unwrap_or(false);

    let body = response.text().map_err(|e| e.to_string())?;
    let count = serde_json::from_str::<Vec<serde_json::Value>>(&body)
        .map(|v| v.len() as u64)
        .unwrap_or(0);

    Ok((count, truncated))
}

fn fetch_latest_workflow_run(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
) -> Result<Option<WorkflowRunResponse>, String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/actions/runs?per_page=1");
    let response = get(client, &url)?;
    if !response.status().is_success() {
        return Ok(None);
    }

    let body = response.text().map_err(|e| e.to_string())?;
    let parsed = parse_workflow_runs(&body).ok();
    Ok(parsed.and_then(|r| r.workflow_runs.into_iter().next()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures shaped exactly like real GitHub API responses (trimmed to
    // the fields this module actually reads).

    const REPO_JSON: &str = r#"{
        "html_url": "https://github.com/octocat/hello-world",
        "stargazers_count": 1500,
        "forks_count": 300,
        "open_issues_count": 12,
        "default_branch": "main",
        "visibility": "public",
        "archived": false
    }"#;

    const RELEASE_JSON: &str = r#"{
        "tag_name": "v1.2.0",
        "name": "Version 1.2.0",
        "published_at": "2026-01-15T10:00:00Z",
        "html_url": "https://github.com/octocat/hello-world/releases/tag/v1.2.0"
    }"#;

    const WORKFLOW_RUNS_JSON: &str = r#"{
        "total_count": 42,
        "workflow_runs": [
            {
                "name": "CI",
                "status": "completed",
                "conclusion": "success",
                "html_url": "https://github.com/octocat/hello-world/actions/runs/1",
                "created_at": "2026-01-20T08:00:00Z"
            }
        ]
    }"#;

    #[test]
    fn parses_a_real_shaped_repo_response() {
        let repo = parse_repo(REPO_JSON).unwrap();
        assert_eq!(repo.stargazers_count, 1500);
        assert_eq!(repo.forks_count, 300);
        assert_eq!(repo.default_branch, "main");
        assert!(!repo.archived);
    }

    #[test]
    fn parses_a_real_shaped_release_response() {
        let release = parse_release(RELEASE_JSON).unwrap();
        assert_eq!(release.tag_name, "v1.2.0");
    }

    const RELEASES_JSON: &str = r#"[
        {
            "tag_name": "v2.0.0",
            "name": "Version 2.0.0",
            "published_at": "2026-03-01T10:00:00Z",
            "html_url": "https://github.com/octocat/hello-world/releases/tag/v2.0.0"
        },
        {
            "tag_name": "v1.0.0",
            "name": null,
            "published_at": "2026-01-01T10:00:00Z",
            "html_url": "https://github.com/octocat/hello-world/releases/tag/v1.0.0"
        }
    ]"#;

    #[test]
    fn parses_a_real_shaped_releases_array_in_order() {
        let releases = parse_releases(RELEASES_JSON).unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag_name, "v2.0.0");
        assert_eq!(releases[0].name.as_deref(), Some("Version 2.0.0"));
        assert_eq!(releases[1].tag_name, "v1.0.0");
        assert_eq!(releases[1].name, None);
    }

    #[test]
    fn empty_releases_array_is_not_an_error() {
        let releases = parse_releases("[]").unwrap();
        assert!(releases.is_empty());
    }

    #[test]
    fn malformed_releases_json_is_an_error_not_a_panic() {
        assert!(parse_releases("not json").is_err());
        assert!(parse_releases(r#"{"not": "an array"}"#).is_err());
    }

    #[test]
    fn map_release_preserves_every_field() {
        let releases = parse_releases(RELEASES_JSON).unwrap();
        let mapped = map_release(releases.into_iter().next().unwrap());
        assert_eq!(mapped.tag_name, "v2.0.0");
        assert_eq!(mapped.name.as_deref(), Some("Version 2.0.0"));
        assert_eq!(mapped.published_at.as_deref(), Some("2026-03-01T10:00:00Z"));
        assert_eq!(mapped.html_url, "https://github.com/octocat/hello-world/releases/tag/v2.0.0");
    }

    #[test]
    fn parses_a_real_shaped_workflow_runs_response() {
        let runs = parse_workflow_runs(WORKFLOW_RUNS_JSON).unwrap();
        assert_eq!(runs.workflow_runs.len(), 1);
        assert_eq!(runs.workflow_runs[0].conclusion.as_deref(), Some("success"));
    }

    #[test]
    fn assembles_full_github_data_from_all_four_responses() {
        let repo = parse_repo(REPO_JSON).unwrap();
        let release = parse_release(RELEASE_JSON).unwrap();
        let runs = parse_workflow_runs(WORKFLOW_RUNS_JSON).unwrap();
        let run = runs.workflow_runs.into_iter().next();

        let data = assemble("octocat", "hello-world", repo, Some(release), 3, false, run);

        assert_eq!(data.owner, "octocat");
        assert_eq!(data.stars, 1500);
        assert_eq!(data.open_pull_requests, 3);
        assert!(!data.open_pull_requests_truncated);
        assert_eq!(data.latest_release.unwrap().tag_name, "v1.2.0");
        assert_eq!(data.latest_workflow_run.unwrap().conclusion.as_deref(), Some("success"));
    }

    #[test]
    fn no_release_is_none_not_an_error() {
        let repo = parse_repo(REPO_JSON).unwrap();
        let data = assemble("octocat", "hello-world", repo, None, 0, false, None);
        assert!(data.latest_release.is_none());
    }

    #[test]
    fn missing_visibility_field_defaults_to_public() {
        let json = r#"{
            "html_url": "https://github.com/octocat/hello-world",
            "stargazers_count": 0,
            "forks_count": 0,
            "open_issues_count": 0,
            "default_branch": "main"
        }"#;
        let repo = parse_repo(json).unwrap();
        assert_eq!(repo.visibility, None);
        let data = assemble("octocat", "hello-world", repo, None, 0, false, None);
        assert_eq!(data.visibility, "public");
    }

    #[test]
    fn malformed_json_is_a_clear_error_not_a_panic() {
        let result = parse_repo("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn no_token_means_no_authorization_header() {
        let headers = build_headers(None).unwrap();
        assert!(!headers.contains_key(reqwest::header::AUTHORIZATION));
        assert_eq!(
            headers.get(reqwest::header::ACCEPT).unwrap(),
            "application/vnd.github+json"
        );
    }

    #[test]
    fn a_token_produces_a_bearer_authorization_header() {
        let headers = build_headers(Some("ghp_fake_token_for_test")).unwrap();
        assert_eq!(
            headers.get(reqwest::header::AUTHORIZATION).unwrap(),
            "Bearer ghp_fake_token_for_test"
        );
    }

    #[test]
    fn rate_limit_message_mentions_the_right_limit_for_each_mode() {
        let unauthenticated = rate_limit_message(false);
        let authenticated = rate_limit_message(true);

        assert!(unauthenticated.contains("60 requests/hour"));
        assert!(authenticated.contains("5,000 requests/hour"));
        assert_ne!(unauthenticated, authenticated);
    }

    #[test]
    fn build_headers_rejects_a_token_with_invalid_header_characters() {
        // Header values can't contain control characters like newlines.
        let result = build_headers(Some("abc\ndef"));
        assert!(result.is_err());
    }

    fn candidate() -> InboxCandidate {
        InboxCandidate {
            repo_id: "abc123".to_string(),
            repo_name: "slate".to_string(),
            owner: "acme".to_string(),
            repo: "slate".to_string(),
        }
    }

    fn run(status: &str, conclusion: Option<&str>) -> GitHubWorkflowRun {
        GitHubWorkflowRun {
            name: "CI".to_string(),
            status: status.to_string(),
            conclusion: conclusion.map(str::to_string),
            html_url: "https://github.com/acme/slate/actions/runs/1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            branch: Some("main".to_string()),
            event: Some("push".to_string()),
        }
    }

    #[test]
    fn completed_failure_produces_an_inbox_item() {
        let item = build_inbox_item(&candidate(), &run("completed", Some("failure")));
        assert!(item.is_some());
        assert_eq!(item.unwrap().repo_name, "slate");
    }

    #[test]
    fn completed_success_produces_no_inbox_item() {
        assert!(build_inbox_item(&candidate(), &run("completed", Some("success"))).is_none());
    }

    #[test]
    fn in_progress_run_produces_no_inbox_item_even_if_it_might_fail_later() {
        assert!(build_inbox_item(&candidate(), &run("in_progress", None)).is_none());
    }

    #[test]
    fn cancelled_run_produces_no_inbox_item() {
        assert!(build_inbox_item(&candidate(), &run("completed", Some("cancelled"))).is_none());
    }

    // --- Live-network tests, not run by default. ---
    //
    // Run explicitly with: cargo test -p haleacas-core --features
    // (none needed) -- --ignored
    //
    // These hit the real GitHub API. They exist so this integration was
    // actually verified against GitHub at least once, not just against
    // fixtures I wrote by hand.

    #[test]
    #[ignore = "hits the real GitHub API — run manually with `cargo test -- --ignored`"]
    fn live_fetch_of_a_known_public_repo_succeeds() {
        let data = fetch("octocat", "Hello-World", None).expect("fetch should succeed");
        assert_eq!(data.owner, "octocat");
        assert!(data.stars > 0);
    }

    #[test]
    #[ignore = "hits the real GitHub API — run manually with `cargo test -- --ignored`"]
    fn live_fetch_of_a_nonexistent_repo_returns_a_clear_error() {
        let result = fetch("this-owner-should-not-exist-12345", "nor-this-repo-6789", None);
        assert!(result.is_err());
        let message = result.unwrap_err();
        // Ideally this is specifically the 404 "not found" message — but
        // unauthenticated GitHub API access is capped at 60 requests/hour
        // per IP, and this test suite makes several live calls per run,
        // so a run that lands after the quota is already spent will see
        // "rate limit reached" instead. Both are the correct, intended
        // error path (a real status code mapped to a real, clear
        // message) — this assertion accepts either rather than being
        // flaky against a quota this code doesn't control.
        assert!(
            message.contains("not found") || message.contains("rate limit"),
            "unexpected error message: {message}"
        );
    }
}
