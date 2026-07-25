//! Scope fingerprints: repo / user / org.

use crate::content_hash::content_hash_hex;

/// Scope dimension for memory isolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Repo,
    User,
    Org,
}

impl ScopeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repo => "repo",
            Self::User => "user",
            Self::Org => "org",
        }
    }
}

/// Hash `kind` + normalized `id` into a stable scope key.
#[must_use]
pub fn scope_hash(kind: ScopeKind, id: &str) -> String {
    let norm = id.trim().to_ascii_lowercase();
    let material = format!("{}:{norm}", kind.as_str());
    content_hash_hex(material.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_kinds() {
        let a = scope_hash(ScopeKind::Repo, "Acme/App");
        let b = scope_hash(ScopeKind::User, "Acme/App");
        assert_ne!(a, b);
        assert_eq!(a, scope_hash(ScopeKind::Repo, "acme/app"));
    }
}
