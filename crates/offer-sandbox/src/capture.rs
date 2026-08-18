//! Per-exec stdout/stderr capture caps (`sak553-b`).

pub(crate) const DEFAULT_CAPTURE_BYTES: usize = 1_048_576;

pub(crate) fn capture_cap(policy: Option<u64>) -> usize {
    policy
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(DEFAULT_CAPTURE_BYTES)
}

pub(crate) fn truncate_capture(raw: &str, cap: usize) -> (String, bool) {
    let bytes = raw.as_bytes();
    if bytes.len() <= cap {
        return (raw.to_string(), false);
    }
    let mut end = cap;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    (raw[..end].to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_boundary_not_split() {
        let (out, truncated) = truncate_capture("éé", 1);
        assert!(truncated);
        assert!(out.is_empty() || out.is_char_boundary(out.len()));
        assert!(out.len() <= 1);
    }

    #[test]
    fn under_cap_passthrough() {
        let (out, truncated) = truncate_capture("hi", 8);
        assert_eq!(out, "hi");
        assert!(!truncated);
    }
}
