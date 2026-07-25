//! Shared in-process memory plane (index + fingerprint + scope).

use std::sync::Mutex;

use crate::fingerprint::{fingerprint_matches, index_fingerprint};
use crate::hash_embed::hash_embed;
use crate::naive_index::SearchHit;
use crate::vector::{BackendKind, DynIndex, VectorIndex};
use crate::{chunk_text, content_hash_hex, ChunkParams};

const EMBED_DIMS: usize = 32;

/// Mutable memory index state shared by `memory.index` / `memory.search`.
#[derive(Debug)]
pub struct MemoryState {
    pub index: DynIndex,
    pub fingerprint: String,
    pub scope_key: String,
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            index: DynIndex::new(BackendKind::Exact),
            fingerprint: String::new(),
            scope_key: String::new(),
        }
    }
}

/// Process-local plane behind offers.
#[derive(Debug, Default)]
pub struct MemoryPlane {
    pub(crate) state: Mutex<MemoryState>,
}

impl MemoryPlane {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Select backend (clears index when kind changes).
    ///
    /// # Panics
    /// Panics if the memory mutex is poisoned.
    pub fn set_backend(&self, kind: BackendKind) {
        let mut g = self.state.lock().expect("memory lock");
        if g.index.kind() != kind {
            g.index = DynIndex::new(kind);
            g.fingerprint.clear();
        }
    }

    /// Rebuild from documents; skip when fingerprint unchanged.
    ///
    /// Returns `(rebuilt, vector_count, fingerprint)`.
    ///
    /// # Errors
    /// Returns an error string if the memory mutex is poisoned.
    pub fn rebuild(
        &self,
        docs: &[(String, String)],
        scope_key: &str,
    ) -> Result<(bool, usize, String), String> {
        let mut hashes = Vec::with_capacity(docs.len());
        let mut prepared = Vec::with_capacity(docs.len());
        for (id, text) in docs {
            let chunks = chunk_text(
                text,
                ChunkParams {
                    size: 256,
                    overlap: 32,
                },
            );
            if chunks.is_empty() {
                let h = content_hash_hex(text.as_bytes());
                hashes.push(h.clone());
                prepared.push((id.clone(), text.clone(), hash_embed(text, EMBED_DIMS)));
            } else {
                for ch in chunks {
                    hashes.push(ch.hash.clone());
                    let cid = format!("{}#{}", id, ch.index);
                    prepared.push((cid, ch.text.clone(), hash_embed(&ch.text, EMBED_DIMS)));
                }
            }
        }
        let fp = index_fingerprint(&hashes);
        let mut g = self.state.lock().map_err(|_| "memory lock".to_string())?;
        if fingerprint_matches(&g.fingerprint, &fp) && g.scope_key == scope_key {
            return Ok((false, g.index.len(), fp));
        }
        g.index.clear();
        for (id, text, vec) in prepared {
            g.index.upsert(id, vec, text);
        }
        g.fingerprint.clone_from(&fp);
        scope_key.clone_into(&mut g.scope_key);
        Ok((true, g.index.len(), fp))
    }

    /// Search the shared index.
    ///
    /// # Errors
    /// Returns an error string if the memory mutex is poisoned.
    pub fn search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>, String> {
        let q = hash_embed(query, EMBED_DIMS);
        let g = self.state.lock().map_err(|_| "memory lock".to_string())?;
        Ok(g.index.search(&q, k.max(1)))
    }

    /// Snapshot fingerprint, backend id, and vector count.
    ///
    /// # Errors
    /// Returns an error string if the memory mutex is poisoned.
    pub fn meta(&self) -> Result<(String, String, usize), String> {
        let g = self.state.lock().map_err(|_| "memory lock".to_string())?;
        Ok((
            g.fingerprint.clone(),
            g.index.backend_id().to_owned(),
            g.index.len(),
        ))
    }
}

/// Truncate text for search excerpts.
#[must_use]
pub fn excerpt(text: &str, max_chars: usize) -> String {
    let max_chars = max_chars.max(16);
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuild_skip_and_search() {
        let plane = MemoryPlane::new();
        let docs = vec![
            ("1".into(), "rust memory index".into()),
            ("2".into(), "python list sort".into()),
        ];
        let (built, n, fp) = plane.rebuild(&docs, "scope-a").expect("rebuild");
        assert!(built);
        assert!(n >= 2);
        let (built2, _, fp2) = plane.rebuild(&docs, "scope-a").expect("skip");
        assert!(!built2);
        assert_eq!(fp, fp2);
        let hits = plane.search("rust memory", 1).expect("search");
        assert_eq!(hits[0].id.chars().next(), Some('1'));
    }
}
