use crate::state::AppState;
use haleacas_core::ai::{client, context};
use haleacas_core::db;
use haleacas_core::dependencies;
use haleacas_core::explorer;
use haleacas_core::health::doctor;
use haleacas_core::types::{AIContext, AIResponse, AISettings};
use std::path::Path;
use tauri::State;

/// "Test Connection" (spec section 19). Takes the settings directly
/// from the frontend rather than reading them from the database, so the
/// user can test an endpoint they've typed but not saved yet — same
/// reasoning as `connect_github_token` not pre-validating before store,
/// just the opposite direction (here, validating *before* save is the
/// whole point of the button). A real network call, so it runs on a
/// blocking thread rather than stalling the async IPC runtime.
#[tauri::command]
pub async fn test_ai_connection(settings: AISettings) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || client::test_connection(&settings))
        .await
        .map_err(|e| e.to_string())?
}

/// Ask the configured local AI provider a question about one
/// repository (spec section 18). `context` controls exactly what's
/// sent on top of the always-included Repository Facts baseline — see
/// `ai::context` module docs for why "Selected diff" and "Full
/// repository" aren't options here yet.
///
/// Deliberately re-checks `settings.ai.enabled()` here even though the
/// frontend should already prevent reaching this command in an
/// unconfigured state — the same defense-in-depth principle
/// `commands::security`'s ignore-list filter applies everywhere a
/// fingerprint gets rebuilt, applied here to "don't attempt a request
/// with no endpoint configured."
#[tauri::command]
pub async fn ai_ask(
    id: String,
    context: AIContext,
    question: String,
    state: State<'_, AppState>,
) -> Result<AIResponse, String> {
    let question = question.trim().to_string();
    if question.is_empty() {
        return Err("Ask something first.".to_string());
    }

    let (fp, settings) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let fp = db::repositories::get_by_id(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;
        let settings = db::settings::load(&conn).map_err(|e| e.to_string())?;
        (fp, settings)
    };

    if !settings.ai.enabled() {
        return Err("AI is not set up yet. Pick a provider and endpoint in Settings.".to_string());
    }

    let findings = doctor::diagnose(&fp);
    let deps = dependencies::scan(Path::new(&fp.path));

    let files = match &context {
        AIContext::RepositoryFacts => Vec::new(),
        AIContext::SelectedFiles { paths } => {
            if paths.is_empty() {
                return Err("No files selected.".to_string());
            }
            let root = Path::new(&fp.path);
            let mut collected = Vec::with_capacity(paths.len());
            for path in paths {
                let (content, truncated) = explorer::read_file_for_context(root, path)?;
                collected.push((path.clone(), content, truncated));
            }
            collected
        }
    };

    let summary = context::build_context(&context, &fp, &findings, &deps, &files);

    let ai_settings = settings.ai;
    let context_text = summary.text.clone();
    let answer = tauri::async_runtime::spawn_blocking(move || {
        client::send_chat(&ai_settings, &context_text, &question)
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(AIResponse {
        context: summary,
        answer,
    })
}
