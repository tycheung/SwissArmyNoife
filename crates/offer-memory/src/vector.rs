//! Pluggable vector index backends.

use crate::naive_index::{NaiveIndex, SearchHit};

/// Backend identifier used in bind policy / catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    Exact,
    Hnsw,
    /// Catalogued FAISS provider; OSS build uses exact stand-in until FFI (`sak222b-b`).
    Faiss,
}

impl BackendKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Hnsw => "hnsw",
            Self::Faiss => "faiss",
        }
    }

    /// Parse `exact` | `hnsw` | `faiss` (case-insensitive). Unknown → [`None`].
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "exact" | "naive" => Some(Self::Exact),
            "hnsw" => Some(Self::Hnsw),
            "faiss" => Some(Self::Faiss),
            _ => None,
        }
    }
}

/// Common upsert/search surface for memory indexes.
pub trait VectorIndex: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn clear(&mut self);
    fn upsert(&mut self, id: String, vector: Vec<f32>, text: String);
    fn search(&self, query: &[f32], k: usize) -> Vec<SearchHit>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl VectorIndex for NaiveIndex {
    fn backend_id(&self) -> &'static str {
        BackendKind::Exact.as_str()
    }

    fn clear(&mut self) {
        *self = Self::new();
    }

    fn upsert(&mut self, id: String, vector: Vec<f32>, text: String) {
        NaiveIndex::upsert(self, id, vector, text);
    }

    fn search(&self, query: &[f32], k: usize) -> Vec<SearchHit> {
        NaiveIndex::search(self, query, k)
    }

    fn len(&self) -> usize {
        NaiveIndex::len(self)
    }
}

/// Enum wrapper so offers can switch backends at bind time.
#[derive(Clone, Debug)]
pub enum DynIndex {
    Exact(NaiveIndex),
    Hnsw(crate::hnsw_lite::HnswLite),
    /// Stand-in until real FAISS FFI is linked.
    Faiss(NaiveIndex),
}

impl DynIndex {
    #[must_use]
    pub fn new(kind: BackendKind) -> Self {
        match kind {
            BackendKind::Exact => Self::Exact(NaiveIndex::new()),
            BackendKind::Hnsw => Self::Hnsw(crate::hnsw_lite::HnswLite::new(8)),
            BackendKind::Faiss => Self::Faiss(NaiveIndex::new()),
        }
    }

    #[must_use]
    pub fn kind(&self) -> BackendKind {
        match self {
            Self::Exact(_) => BackendKind::Exact,
            Self::Hnsw(_) => BackendKind::Hnsw,
            Self::Faiss(_) => BackendKind::Faiss,
        }
    }
}

impl VectorIndex for DynIndex {
    fn backend_id(&self) -> &'static str {
        self.kind().as_str()
    }

    fn clear(&mut self) {
        match self {
            Self::Exact(i) | Self::Faiss(i) => i.clear(),
            Self::Hnsw(i) => i.clear(),
        }
    }

    fn upsert(&mut self, id: String, vector: Vec<f32>, text: String) {
        match self {
            Self::Exact(i) | Self::Faiss(i) => i.upsert(id, vector, text),
            Self::Hnsw(i) => i.upsert(id, vector, text),
        }
    }

    fn search(&self, query: &[f32], k: usize) -> Vec<SearchHit> {
        match self {
            Self::Exact(i) | Self::Faiss(i) => i.search(query, k),
            Self::Hnsw(i) => i.search(query, k),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Exact(i) | Self::Faiss(i) => i.len(),
            Self::Hnsw(i) => i.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_backend() {
        assert_eq!(BackendKind::parse("HNSW"), Some(BackendKind::Hnsw));
        assert_eq!(BackendKind::parse("exact"), Some(BackendKind::Exact));
        assert_eq!(BackendKind::parse("faiss"), Some(BackendKind::Faiss));
        assert!(BackendKind::parse("annoy").is_none());
    }

    /// `BackendKind::Faiss` still uses NaiveIndex stand-in (`sak222b-f`).
    #[test]
    fn faiss_backend_uses_naive_fallback() {
        let mut idx = DynIndex::new(BackendKind::Faiss);
        assert_eq!(idx.kind(), BackendKind::Faiss);
        assert_eq!(idx.backend_id(), "faiss");
        idx.upsert("a".into(), vec![1.0, 0.0], "alpha".into());
        let hits = idx.search(&[1.0, 0.0], 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");
    }

    /// Empty `faiss-ffi` feature compiles without linking native FAISS (`sak222b-g`).
    #[test]
    #[cfg(feature = "faiss-ffi")]
    fn faiss_ffi_feature_still_naive_fallback() {
        assert_eq!(BackendKind::parse("faiss"), Some(BackendKind::Faiss));
        let idx = DynIndex::new(BackendKind::Faiss);
        assert_eq!(idx.kind(), BackendKind::Faiss);
        assert!(idx.is_empty());
    }
}
