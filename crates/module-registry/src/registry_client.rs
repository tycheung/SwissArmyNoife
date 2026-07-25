//! Registry HTTPS client contract (`sak364`).

use std::path::Path;

use serde::{Deserialize, Serialize};
use types::ErrorCode;

/// Resolved module artifact metadata from a registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedModule {
    pub id: String,
    pub version: String,
    pub download_url: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// Registry resolve + download surface.
pub trait RegistryClient: Send + Sync {
    /// Resolve `id@version` (or latest if version empty).
    fn resolve(
        &self,
        id: &str,
        version: &str,
    ) -> impl std::future::Future<Output = Result<ResolvedModule, ErrorCode>> + Send;

    /// Download bytes from a resolved URL.
    fn download(
        &self,
        url: &str,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ErrorCode>> + Send;
}

/// `reqwest`-backed registry client.
#[derive(Clone, Debug)]
pub struct HttpRegistryClient {
    base: String,
    http: reqwest::Client,
}

impl HttpRegistryClient {
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base: base_url.into().trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }
}

impl RegistryClient for HttpRegistryClient {
    async fn resolve(&self, id: &str, version: &str) -> Result<ResolvedModule, ErrorCode> {
        let url = if version.is_empty() {
            format!("{}/v1/modules/{id}", self.base)
        } else {
            format!("{}/v1/modules/{id}/{version}", self.base)
        };
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?;
        if !resp.status().is_success() {
            return Err(ErrorCode::OfferNotFound);
        }
        resp.json::<ResolvedModule>()
            .await
            .map_err(|_| ErrorCode::SchemaInvalid)
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>, ErrorCode> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?;
        if !resp.status().is_success() {
            return Err(ErrorCode::ProviderUnreachable);
        }
        Ok(resp
            .bytes()
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?
            .to_vec())
    }
}

/// Write downloaded bytes to `dest` path.
///
/// # Errors
/// I/O → schema invalid.
pub fn write_download(dest: &Path, bytes: &[u8]) -> Result<(), ErrorCode> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|_| ErrorCode::SchemaInvalid)?;
    }
    std::fs::write(dest, bytes).map_err(|_| ErrorCode::SchemaInvalid)
}

/// In-memory fake for tests / offline.
#[derive(Clone, Debug, Default)]
pub struct FakeRegistryClient {
    pub resolved: Option<ResolvedModule>,
    pub body: Vec<u8>,
}

impl RegistryClient for FakeRegistryClient {
    async fn resolve(&self, id: &str, version: &str) -> Result<ResolvedModule, ErrorCode> {
        let Some(r) = &self.resolved else {
            return Err(ErrorCode::OfferNotFound);
        };
        if r.id != id {
            return Err(ErrorCode::OfferNotFound);
        }
        if !version.is_empty() && r.version != version {
            return Err(ErrorCode::OfferNotFound);
        }
        Ok(r.clone())
    }

    async fn download(&self, _url: &str) -> Result<Vec<u8>, ErrorCode> {
        Ok(self.body.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_resolve_download() {
        let client = FakeRegistryClient {
            resolved: Some(ResolvedModule {
                id: "community.echo".into(),
                version: "0.1.0".into(),
                download_url: "https://example.com/echo.tgz".into(),
                sha256: None,
            }),
            body: b"tarball-bytes".to_vec(),
        };
        let r = client.resolve("community.echo", "0.1.0").await.unwrap();
        assert_eq!(r.version, "0.1.0");
        let bytes = client.download(&r.download_url).await.unwrap();
        assert_eq!(bytes, b"tarball-bytes");
    }
}
