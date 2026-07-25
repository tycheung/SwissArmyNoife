//! Content addressing for memory chunks.

use sha2::{Digest, Sha256};

/// SHA-256 hex digest of `bytes` (lowercase).
#[must_use]
pub fn content_hash_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for b in out {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_hash() {
        let a = content_hash_hex(b"hello");
        let b = content_hash_hex(b"hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert_ne!(a, content_hash_hex(b"world"));
    }
}
