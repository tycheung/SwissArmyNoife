//! Embed request/response wire shapes (provider-facing).

use serde::{Deserialize, Serialize};

/// Embedding request for one or more input strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub model: String,
    pub inputs: Vec<String>,
}

/// Embedding response: one vector per input (same order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmbedResponse {
    pub vectors: Vec<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}
