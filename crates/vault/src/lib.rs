//! Secret vault for `SwissArmyNoife` (ChaCha20-Poly1305).
//!
//! # Environment
//!
//! | Variable | Purpose |
//! |----------|---------|
//! | `VAULT_KEY` | 64 hex chars (32-byte key). If unset, [`VaultKey::generate`] makes an ephemeral key. |

use std::fmt;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// `VAULT_KEY` — 32-byte key as 64 lowercase/uppercase hex characters.
pub const VAULT_KEY: &str = "VAULT_KEY";

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Vault errors.
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("invalid vault key: {0}")]
    InvalidKey(&'static str),
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
    #[error("ciphertext too short")]
    CiphertextTooShort,
}

/// 32-byte vault key (redacted in `Debug`).
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultKey([u8; KEY_LEN]);

impl fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VaultKey([REDACTED])")
    }
}

impl VaultKey {
    /// Load from `VAULT_KEY` or generate a random ephemeral key.
    ///
    /// # Errors
    /// Returns [`VaultError::InvalidKey`] when the env value is present but malformed.
    pub fn bootstrap() -> Result<Self, VaultError> {
        match std::env::var(VAULT_KEY) {
            Ok(hex_key) => Self::from_hex(&hex_key),
            Err(_) => Ok(Self::generate()),
        }
    }

    /// Cryptographically random key.
    #[must_use]
    pub fn generate() -> Self {
        let mut key = [0_u8; KEY_LEN];
        rand::thread_rng().fill_bytes(&mut key);
        Self(key)
    }

    /// Parse a 64-character hex key.
    ///
    /// # Errors
    /// Returns [`VaultError::InvalidKey`] on bad length or non-hex input.
    pub fn from_hex(hex_key: &str) -> Result<Self, VaultError> {
        let hex_key = hex_key.trim();
        if hex_key.len() != KEY_LEN * 2 {
            return Err(VaultError::InvalidKey("expected 64 hex characters"));
        }
        let mut key = [0_u8; KEY_LEN];
        for (i, chunk) in hex_key.as_bytes().chunks(2).enumerate() {
            let s = std::str::from_utf8(chunk).map_err(|_| VaultError::InvalidKey("utf8"))?;
            key[i] = u8::from_str_radix(s, 16).map_err(|_| VaultError::InvalidKey("hex"))?;
        }
        Ok(Self(key))
    }
}

/// Secret UTF-8 string that never prints plaintext via `Debug` / `Display`.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Encrypt plaintext; output is `nonce || ciphertext+tag`.
///
/// # Errors
/// Returns [`VaultError::Encrypt`] on AEAD failure.
pub fn encrypt(key: &VaultKey, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
    let cipher = ChaCha20Poly1305::new_from_slice(&key.0).map_err(|_| VaultError::Encrypt)?;
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut out = Vec::with_capacity(NONCE_LEN + plaintext.len() + 16);
    out.extend_from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| VaultError::Encrypt)?;
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt a blob produced by [`encrypt`].
///
/// # Errors
/// Returns [`VaultError`] on short input or AEAD failure.
pub fn decrypt(key: &VaultKey, blob: &[u8]) -> Result<Vec<u8>, VaultError> {
    if blob.len() < NONCE_LEN + 16 {
        return Err(VaultError::CiphertextTooShort);
    }
    let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
    let cipher = ChaCha20Poly1305::new_from_slice(&key.0).map_err(|_| VaultError::Decrypt)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ct).map_err(|_| VaultError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = VaultKey::generate();
        let pt = b"provider-api-key-value";
        let blob = encrypt(&key, pt).expect("encrypt");
        let out = decrypt(&key, &blob).expect("decrypt");
        assert_eq!(out, pt);
    }

    #[test]
    fn bootstrap_from_hex_env() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        std::env::set_var(VAULT_KEY, hex);
        let key = VaultKey::bootstrap().expect("bootstrap");
        let blob = encrypt(&key, b"x").expect("encrypt");
        assert_eq!(decrypt(&key, &blob).expect("decrypt"), b"x");
        std::env::remove_var(VAULT_KEY);
    }

    #[test]
    fn debug_redacts_secret_and_key() {
        let secret = SecretString::new("super-secret-token");
        let key = VaultKey::generate();
        let secret_dbg = format!("{secret:?}");
        let key_dbg = format!("{key:?}");
        let secret_disp = format!("{secret}");
        assert!(!secret_dbg.contains("super-secret-token"));
        assert!(!secret_disp.contains("super-secret-token"));
        assert!(secret_dbg.contains("REDACTED"));
        assert!(key_dbg.contains("REDACTED"));
        assert!(!key_dbg.contains(&format!("{:?}", key.0)));
    }

    #[test]
    fn wrong_key_fails_decrypt() {
        let a = VaultKey::generate();
        let b = VaultKey::generate();
        let blob = encrypt(&a, b"hello").expect("encrypt");
        assert!(decrypt(&b, &blob).is_err());
    }
}
