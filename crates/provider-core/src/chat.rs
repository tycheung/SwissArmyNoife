//! Chat request/response wire shapes (provider-facing).

use serde::{Deserialize, Serialize};

/// Message role in a chat turn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// One chat message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Chat completion request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional provider prompt-cache key (passthrough; backends may ignore).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
}

/// Token accounting from the provider (best-effort).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

impl TokenUsage {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.prompt_tokens.saturating_add(self.completion_tokens)
    }
}

/// Chat completion response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    #[serde(default)]
    pub usage: TokenUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// One streamed chat delta (`sak137-a`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatChunk {
    /// Incremental text (may be empty on final metadata-only chunks).
    pub delta: String,
    /// True when the provider signals completion.
    #[serde(default)]
    pub done: bool,
}

impl ChatChunk {
    #[must_use]
    pub fn delta(text: impl Into<String>) -> Self {
        Self {
            delta: text.into(),
            done: false,
        }
    }

    #[must_use]
    pub fn final_chunk() -> Self {
        Self {
            delta: String::new(),
            done: true,
        }
    }
}
