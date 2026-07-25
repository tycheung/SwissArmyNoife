//! Deterministic text → vector (no external embedder required for index/search smoke).

use sha2::{Digest, Sha256};

/// Fixed-dimension bag-of-tokens from SHA-256 of each whitespace token.
///
/// Overlapping tokens push vectors closer together (good enough for in-process smoke).
#[must_use]
pub fn hash_embed(text: &str, dims: usize) -> Vec<f32> {
    let dims = dims.max(2);
    let mut out = vec![0.0f32; dims];
    let mut any = false;
    for token in text.split_whitespace() {
        any = true;
        let digest = Sha256::digest(token.as_bytes());
        for (i, byte) in digest.iter().enumerate() {
            let slot = i % dims;
            out[slot] += f32::from(*byte) / 255.0;
        }
    }
    if !any {
        let digest = Sha256::digest(text.as_bytes());
        for (i, byte) in digest.iter().enumerate() {
            let slot = i % dims;
            out[slot] += f32::from(*byte) / 255.0;
        }
    }
    // L2 normalize
    let norm = out.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut out {
            *x /= norm;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_texts_closer() {
        let a = hash_embed("hello world", 16);
        let b = hash_embed("hello world again", 16);
        let c = hash_embed("zzzz totally different", 16);
        let ab: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let ac: f32 = a.iter().zip(&c).map(|(x, y)| x * y).sum();
        assert!(ab > ac);
    }
}
