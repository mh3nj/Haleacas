use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::github;
use haleacas_core::github::InboxCandidate;
use haleacas_core::types::{ActionsReport, GitHubData, GitHubReleasesReport, InboxReport, Settings};
use tauri::State;

/// Load the configured token, if any. A keychain access failure (e.g.
/// no credential store available at all) is treated as "no token" for
/// data-fetching purposes rather than failing the whole request — a
/// user who's never connected GitHub shouldn't see a keychain error just
/// from opening a repository's GitHub tab.
fn load_token() -> Option<String> {
    github::token::load().ok().flatten()
}

/// Fetch GitHub data for a repository, keyed off the GitHub remote slug
/// already parsed at scan time. Authenticated automatically when a
/// token is configured. Runs the (blocking) network calls on a blocking
/// thread so the async IPC runtime isn't stalled — same pattern as
/// `scan_repositories`.
#[tauri::command]
pub async fn get_github_data(id: String, state: State<'_, AppState>) -> Result<GitHubData, String> {
    let slug = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let fp = db::repositories::get_by_id(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;
        fp.git
            .github_slug
            .ok_or_else(|| "This repository has no GitHub remote.".to_string())?
    };

    let (owner, repo) = slug
        .split_once('/')
        .ok_or_else(|| format!("Unexpected GitHub slug: {slug}"))?;
    let (owner, repo) = (owner.to_string(), repo.to_string());

    tauri::async_runtime::spawn_blocking(move || {
        let token = load_token();
        github::fetch(&owner, &repo, token.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The release *list* for the repository's Timeline (spec section 22),
/// distinct from `get_github_data`'s single latest release — lets the
/// Timeline mark which of the repository's tags also have a GitHub
/// Release behind them. Same slug-lookup and error shape as
/// `get_github_data`.
#[tauri::command]
pub async fn get_repository_releases(
    id: String,
    state: State<'_, AppState>,
) -> Result<GitHubReleasesReport, String> {
    let slug = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let fp = db::repositories::get_by_id(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;
        fp.git
            .github_slug
            .ok_or_else(|| "This repository has no GitHub remote.".to_string())?
    };

    let (owner, repo) = slug
        .split_once('/')
        .ok_or_else(|| format!("Unexpected GitHub slug: {slug}"))?;
    let (owner, repo) = (owner.to_string(), repo.to_string());

    tauri::async_runtime::spawn_blocking(move || {
        let token = load_token();
        github::fetch_releases(&owner, &repo, token.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// GitHub Inbox (spec section 21) — checks the most recent CI run for
/// every cached repository with a GitHub remote, capped to leave
/// headroom in GitHub's rate limit for other GitHub-tab views in the
/// same hour (a higher cap when a token is configured, since that
/// budget is 5,000/hour instead of 60/hour). **Not automatic** — only
/// runs when explicitly requested from the frontend, for the same
/// reason.
///
/// Stops at the first request that fails (almost certainly a rate limit
/// or an invalid token) rather than continuing to silently under-report
/// — a partial "no issues found" answer built on a mix of real checks
/// and swallowed errors would be a false claim, which spec section
/// 12/29's spirit rules out just as much here as it does for the
/// security scanner.
#[tauri::command]
pub async fn get_github_inbox(state: State<'_, AppState>) -> Result<InboxReport, String> {
    let candidates: Vec<InboxCandidate> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::repositories::list(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter_map(|fp| {
                let slug = fp.git.github_slug?;
                let (owner, repo) = slug.split_once('/')?;
                Some(InboxCandidate {
                    repo_id: fp.id,
                    repo_name: fp.name,
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                })
            })
            .collect()
    };

    let repos_with_github = candidates.len();

    tauri::async_runtime::spawn_blocking(move || {
        let token = load_token();
        let cap = if token.is_some() {
            github::INBOX_REPO_CAP_AUTHENTICATED
        } else {
            github::INBOX_REPO_CAP
        };
        let capped: Vec<InboxCandidate> = candidates.into_iter().take(cap).collect();

        let mut items = Vec::new();
        let mut repos_checked = 0usize;
        let mut error = None;

        for candidate in &capped {
            match github::fetch_ci_status(&candidate.owner, &candidate.repo, token.as_deref()) {
                Ok(Some(run)) => {
                    repos_checked += 1;
                    if let Some(item) = github::build_inbox_item(candidate, &run) {
                        items.push(item);
                    }
                }
                Ok(None) => {
                    repos_checked += 1;
                }
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }

        let truncated = repos_checked < repos_with_github;

        InboxReport {
            items,
            repos_checked,
            repos_with_github,
            truncated,
            error,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Store a GitHub personal access token in the OS keychain and mark it
/// configured in settings. Does **not** validate the token against
/// GitHub before storing it — an invalid or expired token will surface
/// a clear "GitHub rejected the configured token" error the next time
/// something tries to use it, rather than this command making an extra
/// API call just to check upfront.
#[tauri::command]
pub fn connect_github_token(token: String, state: State<AppState>) -> Result<Settings, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Token cannot be empty.".to_string());
    }

    github::token::store(token)?;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut settings = db::settings::load(&conn).map_err(|e| e.to_string())?;
    settings.github_token_configured = true;
    db::settings::save(&conn, &settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

/// Remove the stored GitHub token ("Disconnect").
#[tauri::command]
pub fn disconnect_github_token(state: State<AppState>) -> Result<Settings, String> {
    github::token::delete()?;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut settings = db::settings::load(&conn).map_err(|e| e.to_string())?;
    settings.github_token_configured = false;
    db::settings::save(&conn, &settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

/// Actions overview: recent workflow runs for every cached repo with a
/// GitHub remote. Only runs when the user asks from the Actions page, so
/// the API budget stays under control. Each repo is fetched on its own,
/// so one missing repo never sinks the whole overview.
#[tauri::command]
pub async fn get_github_actions(state: State<'_, AppState>) -> Result<ActionsReport, String> {
    use haleacas_core::types::ActionsRepoReport;

    let candidates: Vec<InboxCandidate> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::repositories::list(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter_map(|fp| {
                let slug = fp.git.github_slug?;
                let (owner, repo) = slug.split_once('/')?;
                Some(InboxCandidate {
                    repo_id: fp.id,
                    repo_name: fp.name,
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                })
            })
            .collect()
    };

    let repos_with_github = candidates.len();

    tauri::async_runtime::spawn_blocking(move || {
        let token = load_token();
        let cap = if token.is_some() {
            github::INBOX_REPO_CAP_AUTHENTICATED
        } else {
            github::INBOX_REPO_CAP
        };
        let capped: Vec<InboxCandidate> = candidates.into_iter().take(cap).collect();

        let mut repos = Vec::new();
        let mut repos_checked = 0usize;
        let mut error = None;

        for candidate in &capped {
            let html_url = format!(
                "https://github.com/{}/{}",
                candidate.owner, candidate.repo
            );
            match github::fetch_workflow_runs(
                &candidate.owner,
                &candidate.repo,
                token.as_deref(),
            ) {
                Ok(runs) => {
                    repos_checked += 1;
                    repos.push(ActionsRepoReport {
                        repo_id: candidate.repo_id.clone(),
                        repo_name: candidate.repo_name.clone(),
                        owner: candidate.owner.clone(),
                        repo: candidate.repo.clone(),
                        html_url,
                        runs,
                        error: None,
                    });
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("rate limit") || msg.contains("rejected the configured token") {
                        error = Some(msg);
                        break;
                    }
                    repos_checked += 1;
                    repos.push(ActionsRepoReport {
                        repo_id: candidate.repo_id.clone(),
                        repo_name: candidate.repo_name.clone(),
                        owner: candidate.owner.clone(),
                        repo: candidate.repo.clone(),
                        html_url,
                        runs: Vec::new(),
                        error: Some(msg),
                    });
                }
            }
        }

        let truncated = repos_checked < repos_with_github;

        ActionsReport {
            repos,
            repos_checked,
            repos_with_github,
            truncated,
            error,
        }
    })
    .await
    .map_err(|e| e.to_string())
}
