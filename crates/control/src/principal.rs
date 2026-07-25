//! Principal identity: local ambient vs API key (`sak058`).

use serde::{Deserialize, Serialize};

/// How a principal authenticates to the control plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    /// Stdio / local process ambient trust.
    Local,
    /// Bearer / API-key principal.
    ApiKey,
}

impl PrincipalKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::ApiKey => "api_key",
        }
    }
}

/// Stable principal handle used by policy and audit.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Principal {
    pub id: String,
    pub kind: PrincipalKind,
}

impl Principal {
    /// Default local ambient principal.
    #[must_use]
    pub fn local() -> Self {
        Self {
            id: "local".into(),
            kind: PrincipalKind::Local,
        }
    }

    /// API-key backed principal (id is the key id, not the secret).
    #[must_use]
    pub fn api_key(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: PrincipalKind::ApiKey,
        }
    }

    /// Normalize a bind-tool principal string (`sak058-b`).
    ///
    /// - empty / `"local"` → [`Self::local`]
    /// - `"api_key:<id>"` → [`Self::api_key`]
    /// - otherwise → local-named principal with that id
    #[must_use]
    pub fn from_bind_arg(raw: &str) -> Self {
        let s = raw.trim();
        if s.is_empty() || s.eq_ignore_ascii_case("local") {
            return Self::local();
        }
        if let Some(id) = s.strip_prefix("api_key:") {
            let id = id.trim();
            if !id.is_empty() {
                return Self::api_key(id);
            }
        }
        Self {
            id: s.to_owned(),
            kind: PrincipalKind::Local,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl Default for Principal {
    fn default() -> Self {
        Self::local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_and_api_key_kinds() {
        assert_eq!(Principal::local().kind, PrincipalKind::Local);
        assert_eq!(Principal::api_key("k1").kind, PrincipalKind::ApiKey);
        assert_eq!(PrincipalKind::ApiKey.as_str(), "api_key");
    }

    #[test]
    fn from_bind_arg_normalizes() {
        assert_eq!(Principal::from_bind_arg(""), Principal::local());
        assert_eq!(Principal::from_bind_arg("local"), Principal::local());
        assert_eq!(
            Principal::from_bind_arg("api_key:alice"),
            Principal::api_key("alice")
        );
        let named = Principal::from_bind_arg("dev");
        assert_eq!(named.id, "dev");
        assert_eq!(named.kind, PrincipalKind::Local);
    }
}
