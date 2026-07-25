//! API key mint / verify (`sak059`). Secrets are hashed; plaintext returned only at mint.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use types::ErrorCode;
use uuid::Uuid;

use crate::principal::Principal;

/// Metadata for a minted key (no secret material).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub key_id: String,
    pub principal_id: String,
}

/// Persistable row (hash only — no plaintext secret).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyRow {
    pub key_id: String,
    pub hash_hex: String,
    pub principal_id: String,
}

#[derive(Debug)]
struct StoredKey {
    hash_hex: String,
    principal_id: String,
}

/// Process-local API key store (in-memory; hydrate via [`Self::load_rows`]).
#[derive(Debug, Default)]
pub struct ApiKeyStore {
    keys: Mutex<HashMap<String, StoredKey>>,
}

impl ApiKeyStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a new key. Returns `(info, plaintext_secret)` — store the secret once.
    ///
    /// # Errors
    /// Lock failure → schema invalid.
    pub fn mint(&self, principal_id: impl Into<String>) -> Result<(ApiKeyInfo, String), ErrorCode> {
        let principal_id = principal_id.into();
        let key_id = format!("sak_{}", Uuid::new_v4().simple());
        let secret = format!("sk_live_{}", Uuid::new_v4().simple());
        let hash_hex = hash_api_key_secret(&secret);
        let mut guard = self.keys.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        guard.insert(
            key_id.clone(),
            StoredKey {
                hash_hex,
                principal_id: principal_id.clone(),
            },
        );
        Ok((
            ApiKeyInfo {
                key_id,
                principal_id,
            },
            secret,
        ))
    }

    /// Verify a bearer secret; returns the principal on success.
    ///
    /// # Errors
    /// Unknown / mismatched secret → [`ErrorCode::PolicyDenied`].
    pub fn verify(&self, secret: &str) -> Result<Principal, ErrorCode> {
        let want = hash_api_key_secret(secret);
        let guard = self.keys.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        for stored in guard.values() {
            if stored.hash_hex == want {
                return Ok(Principal::api_key(stored.principal_id.clone()));
            }
        }
        Err(ErrorCode::PolicyDenied)
    }

    /// Replace in-memory keys from persisted rows (e.g. `persist-sqlite` hydrate).
    ///
    /// # Errors
    /// Lock failure → schema invalid.
    pub fn load_rows(&self, rows: impl IntoIterator<Item = ApiKeyRow>) -> Result<(), ErrorCode> {
        let mut guard = self.keys.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        guard.clear();
        for row in rows {
            guard.insert(
                row.key_id,
                StoredKey {
                    hash_hex: row.hash_hex,
                    principal_id: row.principal_id,
                },
            );
        }
        Ok(())
    }

    /// Export all rows for persistence (hash only).
    ///
    /// # Errors
    /// Lock failure → schema invalid.
    pub fn export_rows(&self) -> Result<Vec<ApiKeyRow>, ErrorCode> {
        let guard = self.keys.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(guard
            .iter()
            .map(|(key_id, stored)| ApiKeyRow {
                key_id: key_id.clone(),
                hash_hex: stored.hash_hex.clone(),
                principal_id: stored.principal_id.clone(),
            })
            .collect())
    }

    /// Lookup by key id (no secret check).
    ///
    /// # Errors
    /// Missing key → offer not found.
    pub fn get(&self, key_id: &str) -> Result<ApiKeyInfo, ErrorCode> {
        let guard = self.keys.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let stored = guard.get(key_id).ok_or(ErrorCode::OfferNotFound)?;
        Ok(ApiKeyInfo {
            key_id: key_id.to_owned(),
            principal_id: stored.principal_id.clone(),
        })
    }
}

/// SHA-256 hex digest of an API key bearer secret.
#[must_use]
pub fn hash_api_key_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    digest.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::principal::PrincipalKind;

    #[test]
    fn mint_verify_roundtrip() {
        let store = ApiKeyStore::new();
        let (info, secret) = store.mint("alice").unwrap();
        assert!(info.key_id.starts_with("sak_"));
        let p = store.verify(&secret).unwrap();
        assert_eq!(p.id, "alice");
        assert_eq!(p.kind, PrincipalKind::ApiKey);
        assert_eq!(store.verify("sk_live_nope"), Err(ErrorCode::PolicyDenied));
    }

    #[test]
    fn secret_not_stored_plaintext() {
        let store = ApiKeyStore::new();
        let (info, secret) = store.mint("bob").unwrap();
        let guard = store.keys.lock().unwrap();
        let stored = guard.get(&info.key_id).unwrap();
        assert!(!stored.hash_hex.contains(&secret));
        assert_ne!(stored.hash_hex, secret);
    }

    #[test]
    fn export_load_roundtrip() {
        let store = ApiKeyStore::new();
        let (_info, secret) = store.mint("carol").unwrap();
        let rows = store.export_rows().unwrap();
        assert_eq!(rows.len(), 1);
        let store2 = ApiKeyStore::new();
        store2.load_rows(rows).unwrap();
        assert_eq!(store2.verify(&secret).unwrap().id, "carol");
    }
}
