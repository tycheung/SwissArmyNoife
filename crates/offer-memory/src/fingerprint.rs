//! Index fingerprint for rebuild skip (`sak223`).

use crate::content_hash::content_hash_hex;

/// Stable fingerprint over ordered chunk/content hashes.
#[must_use]
pub fn index_fingerprint(hashes: &[String]) -> String {
    let mut joined = String::new();
    for (i, h) in hashes.iter().enumerate() {
        if i > 0 {
            joined.push('\n');
        }
        joined.push_str(h);
    }
    content_hash_hex(joined.as_bytes())
}

/// Whether a rebuild can be skipped.
#[must_use]
pub fn fingerprint_matches(current: &str, incoming: &str) -> bool {
    !current.is_empty() && current == incoming
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_and_match() {
        let a = index_fingerprint(&["h1".into(), "h2".into()]);
        let b = index_fingerprint(&["h1".into(), "h2".into()]);
        assert_eq!(a, b);
        assert!(fingerprint_matches(&a, &b));
        assert!(!fingerprint_matches(&a, "other"));
    }
}
