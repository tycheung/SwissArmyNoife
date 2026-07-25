//! Simple character-window text chunker.

use crate::content_hash::content_hash_hex;

/// One chunk of source text with content hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub index: u32,
    pub text: String,
    pub hash: String,
}

/// Chunking parameters (character windows; UTF-8 safe via char boundaries).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkParams {
    pub size: usize,
    pub overlap: usize,
}

impl Default for ChunkParams {
    fn default() -> Self {
        Self {
            size: 512,
            overlap: 64,
        }
    }
}

/// Split `text` into overlapping windows.
///
/// Empty input yields no chunks. `overlap` is clamped below `size`.
#[must_use]
pub fn chunk_text(text: &str, params: ChunkParams) -> Vec<Chunk> {
    if text.is_empty() || params.size == 0 {
        return Vec::new();
    }
    let overlap = params.overlap.min(params.size.saturating_sub(1));
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut index = 0u32;
    while start < chars.len() {
        let end = (start + params.size).min(chars.len());
        let piece: String = chars[start..end].iter().collect();
        let hash = content_hash_hex(piece.as_bytes());
        out.push(Chunk {
            index,
            text: piece,
            hash,
        });
        index = index.saturating_add(1);
        if end == chars.len() {
            break;
        }
        let step = params.size.saturating_sub(overlap).max(1);
        start = start.saturating_add(step);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_overlap() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let chunks = chunk_text(
            text,
            ChunkParams {
                size: 10,
                overlap: 2,
            },
        );
        assert!(chunks.len() >= 3);
        assert_eq!(chunks[0].text.len(), 10);
        assert_ne!(chunks[0].hash, chunks[1].hash);
    }

    #[test]
    fn empty_is_empty() {
        assert!(chunk_text("", ChunkParams::default()).is_empty());
    }
}
