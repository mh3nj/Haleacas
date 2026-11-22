//! Local AI integration (spec sections 18-19).
//!
//! AI is entirely optional here — every other module in this crate works
//! with `ai` absent, and nothing else depends on this module. What's
//! implemented:
//!
//! - **Providers**: LM Studio, Ollama, and a "Custom" OpenAI-compatible
//!   endpoint (spec section 19). All three speak the same protocol
//!   (`POST {endpoint}/chat/completions`, `GET {endpoint}/models`), so
//!   one client (`client`) covers all of them — the provider kind only
//!   changes which endpoint the Settings UI suggests by default.
//! - **Context**: spec section 18 lists four example context kinds —
//!   "Repository facts", "Selected files", "Selected diff", "Full
//!   repository" — with the explicit instruction to "never silently
//!   send the full repository." Only the first two are built this
//!   round (`context::build_context`). "Selected diff" would need a
//!   real diff-selection UI this round didn't have time for, and "Full
//!   repository" is the one option the spec itself warns against
//!   sending casually — rather than half-build a size-capped version
//!   that still risks surprising a user with what left their machine,
//!   it's left out entirely and documented as a gap (see STATUS.md).
//!   *Every* request includes the Repository Facts baseline (name,
//!   detected tech, Haleacas Health Score, Repository Doctor findings,
//!   dependency summary) regardless of which `AIContext` variant is
//!   requested — this is the "structured repository information
//!   instead of blindly sending the entire repository" spec section 18
//!   asks for.
//! - **Actions**: spec section 18 lists named actions (explain
//!   repository, explain health issues, summarize commits, improve
//!   README wording, ...). Rather than encoding each as a Rust-side
//!   enum variant with its own prompt template, a single `ai_ask`
//!   command (see `app`'s `commands::ai`) takes a free-text question
//!   plus an explicit `AIContext` — the frontend's "quick action"
//!   buttons are just pre-filled questions paired with the right
//!   context. This keeps the backend generic (also gets a real
//!   free-form "ask your own question" mode for free) without losing
//!   anything the spec's fixed action list needs.
//!
//! No API key handling exists in this module. Local providers (LM
//! Studio, Ollama) don't need one; cloud providers are out of scope
//! this round (spec section 19 explicitly separates "local, no API key
//! required" from "cloud providers, optional and disabled by default"
//! — only the former is built).

pub mod client;
pub mod context;
