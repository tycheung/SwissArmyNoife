//! `llm.*` offer helpers (resolve + chat + embed + preflight + stream).

mod chat;
mod chat_providers;
mod echo_chunks;
mod embed;
mod manage;
mod preflight;
mod resolve;
mod stream;
mod telemetry;

pub use chat::LlmChatOffer;
pub use chat_providers::{ChatProviders, EchoChatProvider};
pub use echo_chunks::echo_chunks;
pub use embed::LlmEmbedOffer;
pub use manage::LlmOllamaManageOffer;
pub use preflight::{FitAdvisor, LlmPreflightOffer, NoFitAdvisor, PreflightCandidate};
pub use resolve::{resolve, BindingSource, ConnectionRef, ResolveError, ResolveHint, ResolvedLlm};
pub use stream::collect_chat_stream;
pub use telemetry::{LlmTelemetryOffer, TelemetryRecord};
