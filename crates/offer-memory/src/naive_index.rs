//! In-memory exact-search index (HNSW comes later as sak222).

/// One stored vector with optional payload text.
#[derive(Clone, Debug)]
pub struct IndexedVec {
    pub id: String,
    pub vector: Vec<f32>,
    pub text: String,
}

/// Ranked search hit.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub score: f32,
    pub text: String,
}

/// Brute-force cosine similarity index.
#[derive(Clone, Debug, Default)]
pub struct NaiveIndex {
    rows: Vec<IndexedVec>,
}

impl NaiveIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, id: impl Into<String>, vector: Vec<f32>, text: impl Into<String>) {
        let id = id.into();
        if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
            row.vector = vector;
            row.text = text.into();
            return;
        }
        self.rows.push(IndexedVec {
            id,
            vector,
            text: text.into(),
        });
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Top-k by cosine similarity (descending).
    #[must_use]
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchHit> {
        let mut hits: Vec<_> = self
            .rows
            .iter()
            .filter_map(|row| {
                let score = cosine(&row.vector, query)?;
                Some(SearchHit {
                    id: row.id.clone(),
                    score,
                    text: row.text.clone(),
                })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(k);
        hits
    }
}

pub(crate) fn cosine(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.is_empty() || a.len() != b.len() {
        return None;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        None
    } else {
        Some(dot / denom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ranks_near() {
        let mut idx = NaiveIndex::new();
        idx.upsert("a", vec![1.0, 0.0], "alpha");
        idx.upsert("b", vec![0.0, 1.0], "beta");
        let hits = idx.search(&[0.9, 0.1], 1);
        assert_eq!(hits[0].id, "a");
    }
}
