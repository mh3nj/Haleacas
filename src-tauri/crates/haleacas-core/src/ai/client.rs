//! HTTP client for local, OpenAI-compatible AI providers (spec section
//! 19: LM Studio, Ollama, or a custom endpoint — all three speak the
//! same `POST {endpoint}/chat/completions` / `GET {endpoint}/models`
//! protocol, so one client covers all of them).
//!
//! Same split as the `github` module: pure functions (`endpoint_url`,
//! `build_messages`, `parse_chat_response`, `parse_models_response`)
//! that are unit-tested against hand-written fixtures shaped like real
//! API responses, plus thin HTTP-calling wrappers (`send_chat`,
//! `test_connection`) covered by `#[ignore]`d tests that hit a real
//! local provider — not run by default `cargo test`, since there's no
//! LM Studio/Ollama instance in a normal test environment (including
//! this project's own CI-less sandbox — see STATUS.md), but written so
//! anyone with one running locally can confirm this actually works
//! against a real endpoint rather than only a fixture written by hand.
//!
//! No authentication headers are ever sent — local providers don't use
//! them (spec section 19: "do not require API keys for local
//! providers").

use crate::types::AISettings;
use serde::{Deserialize, Serialize};

/// System prompt sent with every request. Explicitly instructs the
/// model to reason only from the supplied facts — the AI-side half of
/// spec section 18/29's "do not invent" principle the rest of the app
/// already holds itself to for health scores, dependency versions, and
/// framework detection. Prompting alone can't *guarantee* a model won't
/// hallucinate, but it's a real, honest constraint to state rather than
/// silently hoping the model behaves.
pub const SYSTEM_PROMPT: &str = "You are Haleacas' local AI assistant, helping a developer \
    understand one of their own repositories. You have been given factual information Haleacas \
    already collected about this specific repository below. Only reason from those facts — never \
    invent file names, dependency versions, commit history, or any other detail that isn't present \
    in what you were given. If the provided facts don't cover something the developer asked about, \
    say so plainly rather than guessing.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Builds the two-message conversation sent for every request: the
/// fixed system prompt above, then a user message that concatenates the
/// context text (from `ai::context::build_context`) with the actual
/// question. Pure — no network, directly testable.
pub fn build_messages(context_text: &str, question: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("{context_text}\n\n---\n\nQuestion: {question}"),
        },
    ]
}

/// Joins an endpoint (however the user typed it — with or without a
/// trailing slash) with a path suffix like `/chat/completions` or
/// `/models`. Pure and separate specifically so the trailing-slash edge
/// case is directly testable without a real HTTP call.
pub fn endpoint_url(endpoint: &str, suffix: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    format!("{trimmed}{suffix}")
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

/// Extracts the assistant's reply text from a raw chat-completions
/// response body. Both LM Studio and Ollama's OpenAI-compatible layer
/// use this exact `choices[0].message.content` shape.
pub fn parse_chat_response(json: &str) -> Result<String, String> {
    let parsed: ChatCompletionResponse = serde_json::from_str(json)
        .map_err(|e| format!("Unexpected response from the AI provider: {e}"))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "The AI provider returned no response choices.".to_string())
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

/// Extracts model ids from a `GET /models` response body (the same
/// OpenAI-compatible `{"data": [{"id": "..."}]}` shape both providers
/// return).
pub fn parse_models_response(json: &str) -> Result<Vec<String>, String> {
    let parsed: ModelsResponse = serde_json::from_str(json)
        .map_err(|e| format!("Unexpected response from the AI provider: {e}"))?;
    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}

fn build_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

fn unreachable_message(url: &str, e: &reqwest::Error) -> String {
    format!(
        "Could not reach the AI endpoint at {url}. Confirm LM Studio/Ollama (or your custom \
         server) is running and the endpoint in Settings is correct. ({e})"
    )
}

/// "Test Connection" (spec section 19) — a short-timeout `GET /models`
/// against the configured (or about-to-be-saved) endpoint. Returns the
/// model ids the provider currently reports, which the Settings UI can
/// use both to confirm the connection works and to help the user pick
/// a model id without having to know it in advance.
pub fn test_connection(settings: &AISettings) -> Result<Vec<String>, String> {
    if settings.endpoint.trim().is_empty() {
        return Err("No endpoint configured.".to_string());
    }
    let client = build_client(8)?;
    let url = endpoint_url(&settings.endpoint, "/models");
    let response = client
        .get(&url)
        .send()
        .map_err(|e| unreachable_message(&url, &e))?;

    if !response.status().is_success() {
        return Err(format!(
            "The AI provider at {url} responded with an unexpected status: {}",
            response.status()
        ));
    }
    let body = response.text().map_err(|e| e.to_string())?;
    parse_models_response(&body)
}

/// A much longer timeout than `test_connection` — local models
/// (especially CPU-only ones) can genuinely take over a minute to
/// generate a full response, and a timeout here should mean "something
/// is actually wrong," not "the model is still thinking."
const CHAT_TIMEOUT_SECS: u64 = 180;

/// Sends one chat-completion request and returns the assistant's reply
/// text. `context_text` and `question` are combined via
/// `build_messages` — this function does no prompt construction of its
/// own beyond that.
pub fn send_chat(settings: &AISettings, context_text: &str, question: &str) -> Result<String, String> {
    if settings.endpoint.trim().is_empty() {
        return Err("No AI endpoint configured — set one in Settings first.".to_string());
    }
    if settings.model.trim().is_empty() {
        return Err("No model configured — set one in Settings first.".to_string());
    }

    let client = build_client(CHAT_TIMEOUT_SECS)?;
    let url = endpoint_url(&settings.endpoint, "/chat/completions");
    let messages = build_messages(context_text, question);
    let body = ChatRequest {
        model: &settings.model,
        messages: &messages,
        temperature: settings.temperature,
    };

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| unreachable_message(&url, &e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        let snippet: String = text.chars().take(300).collect();
        return Err(format!(
            "The AI provider at {url} responded with status {status}: {snippet}"
        ));
    }

    let text = response.text().map_err(|e| e.to_string())?;
    parse_chat_response(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AIProviderKind;

    fn settings(endpoint: &str, model: &str) -> AISettings {
        AISettings {
            provider: AIProviderKind::Custom,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            temperature: None,
        }
    }

    // --- Pure logic ---

    #[test]
    fn endpoint_url_joins_without_trailing_slash() {
        assert_eq!(
            endpoint_url("http://localhost:1234/v1", "/models"),
            "http://localhost:1234/v1/models"
        );
    }

    #[test]
    fn endpoint_url_strips_a_trailing_slash_before_joining() {
        assert_eq!(
            endpoint_url("http://localhost:1234/v1/", "/models"),
            "http://localhost:1234/v1/models"
        );
    }

    #[test]
    fn endpoint_url_strips_multiple_trailing_slashes_and_surrounding_whitespace() {
        assert_eq!(
            endpoint_url("  http://localhost:11434/v1//  ", "/chat/completions"),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn build_messages_has_system_then_user_with_both_context_and_question() {
        let messages = build_messages("FACT: this is a Rust project.", "What language is this?");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content, SYSTEM_PROMPT);
        assert_eq!(messages[1].role, "user");
        assert!(messages[1].content.contains("FACT: this is a Rust project."));
        assert!(messages[1].content.contains("What language is this?"));
    }

    const CHAT_RESPONSE_JSON: &str = r#"{
        "id": "chatcmpl-abc",
        "object": "chat.completion",
        "choices": [
            {
                "index": 0,
                "message": { "role": "assistant", "content": "This is a Rust project." },
                "finish_reason": "stop"
            }
        ]
    }"#;

    #[test]
    fn parses_a_real_shaped_chat_completion_response() {
        let answer = parse_chat_response(CHAT_RESPONSE_JSON).unwrap();
        assert_eq!(answer, "This is a Rust project.");
    }

    #[test]
    fn chat_response_with_no_choices_is_a_clear_error_not_a_panic() {
        let result = parse_chat_response(r#"{"choices": []}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no response choices"));
    }

    #[test]
    fn malformed_chat_response_is_an_error_not_a_panic() {
        assert!(parse_chat_response("not json").is_err());
        assert!(parse_chat_response(r#"{"unexpected": true}"#).is_err());
    }

    const MODELS_RESPONSE_JSON: &str = r#"{
        "object": "list",
        "data": [
            { "id": "llama-3.1-8b-instruct", "object": "model" },
            { "id": "qwen2.5-coder-7b", "object": "model" }
        ]
    }"#;

    #[test]
    fn parses_a_real_shaped_models_response() {
        let models = parse_models_response(MODELS_RESPONSE_JSON).unwrap();
        assert_eq!(models, vec!["llama-3.1-8b-instruct", "qwen2.5-coder-7b"]);
    }

    #[test]
    fn empty_models_array_is_not_an_error() {
        let models = parse_models_response(r#"{"data": []}"#).unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn malformed_models_response_is_an_error_not_a_panic() {
        assert!(parse_models_response("not json").is_err());
    }

    #[test]
    fn send_chat_refuses_a_blank_endpoint_without_a_network_call() {
        let result = send_chat(&settings("", "some-model"), "facts", "question?");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No AI endpoint configured"));
    }

    #[test]
    fn send_chat_refuses_a_blank_model_without_a_network_call() {
        let result = send_chat(&settings("http://localhost:1234/v1", ""), "facts", "question?");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No model configured"));
    }

    #[test]
    fn test_connection_refuses_a_blank_endpoint_without_a_network_call() {
        let result = test_connection(&settings("", ""));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No endpoint configured"));
    }

    // --- Live-network tests, not run by default. ---
    //
    // Run explicitly with: cargo test -p haleacas-core -- --ignored
    //
    // These hit a real local provider on the default LM Studio port.
    // They exist so this integration is verified against a real
    // OpenAI-compatible server at least once, not just against
    // fixtures written by hand — same reasoning as the `github`
    // module's live tests.

    #[test]
    #[ignore = "hits a real local LM Studio/Ollama server on :1234 — run manually with \
                `cargo test -- --ignored` while one is running"]
    fn live_test_connection_against_a_local_lm_studio_instance() {
        let models = test_connection(&settings("http://localhost:1234/v1", "")).expect(
            "test_connection should succeed against a real LM Studio instance listening on :1234",
        );
        assert!(!models.is_empty(), "expected at least one loaded model");
    }

    #[test]
    #[ignore = "hits a real local LM Studio/Ollama server on :1234 — run manually with \
                `cargo test -- --ignored` while one is running, and edit MODEL to a real loaded model id"]
    fn live_send_chat_against_a_local_lm_studio_instance() {
        const MODEL: &str = "REPLACE_WITH_A_REAL_LOADED_MODEL_ID";
        let answer = send_chat(
            &settings("http://localhost:1234/v1", MODEL),
            "Repository: a test project with no special facts.",
            "Reply with exactly the word: OK",
        )
        .expect("send_chat should succeed against a real LM Studio instance");
        assert!(!answer.trim().is_empty());
    }
}
