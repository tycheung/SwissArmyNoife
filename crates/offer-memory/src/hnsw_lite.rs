//! Lightweight NSW/HNSW-style graph index (pure Rust, no FFI).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use crate::naive_index::{cosine, IndexedVec, SearchHit};
use crate::vector::VectorIndex;

#[derive(Clone, Debug)]
struct Node {
    row: IndexedVec,
    neighbors: Vec<usize>,
}

/// Approximate nearest-neighbor index with fixed out-degree `m`.
#[derive(Clone, Debug)]
pub struct HnswLite {
    m: usize,
    nodes: Vec<Node>,
    entry: Option<usize>,
}

impl HnswLite {
    #[must_use]
    pub fn new(m: usize) -> Self {
        Self {
            m: m.max(2),
            nodes: Vec::new(),
            entry: None,
        }
    }

    fn search_candidates(&self, query: &[f32], ef: usize) -> Vec<(f32, usize)> {
        let Some(entry) = self.entry else {
            return Vec::new();
        };
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut best: Vec<(f32, usize)> = Vec::new();

        let entry_score = cosine(&self.nodes[entry].row.vector, query).unwrap_or(f32::MIN);
        visited.insert(entry);
        candidates.push(Cand {
            score: entry_score,
            idx: entry,
        });
        best.push((entry_score, entry));

        while let Some(Cand { score: sc, idx }) = candidates.pop() {
            let worst = best.last().map_or(f32::MIN, |x| x.0);
            if best.len() >= ef && sc < worst {
                break;
            }
            for &nb in &self.nodes[idx].neighbors {
                if !visited.insert(nb) {
                    continue;
                }
                let nsc = cosine(&self.nodes[nb].row.vector, query).unwrap_or(f32::MIN);
                let worst = best.last().map_or(f32::MIN, |x| x.0);
                if best.len() < ef || nsc > worst {
                    candidates.push(Cand {
                        score: nsc,
                        idx: nb,
                    });
                    best.push((nsc, nb));
                    best.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
                    best.truncate(ef);
                }
            }
        }
        best
    }
}

impl VectorIndex for HnswLite {
    fn backend_id(&self) -> &'static str {
        "hnsw"
    }

    fn clear(&mut self) {
        self.nodes.clear();
        self.entry = None;
    }

    fn upsert(&mut self, id: String, vector: Vec<f32>, text: String) {
        if let Some(pos) = self.nodes.iter().position(|n| n.row.id == id) {
            self.nodes[pos].row.vector = vector;
            self.nodes[pos].row.text = text;
            return;
        }
        let new_idx = self.nodes.len();
        if self.entry.is_none() {
            self.nodes.push(Node {
                row: IndexedVec { id, vector, text },
                neighbors: Vec::new(),
            });
            self.entry = Some(0);
            return;
        }
        let nearest = self.search_candidates(&vector, self.m.max(8));
        let neighbors: Vec<_> = nearest.into_iter().take(self.m).map(|(_, i)| i).collect();
        self.nodes.push(Node {
            row: IndexedVec { id, vector, text },
            neighbors: neighbors.clone(),
        });
        for nb in neighbors {
            if !self.nodes[nb].neighbors.contains(&new_idx) {
                self.nodes[nb].neighbors.push(new_idx);
            }
            if self.nodes[nb].neighbors.len() <= self.m {
                continue;
            }
            let nb_vec = self.nodes[nb].row.vector.clone();
            let mut scored: Vec<(f32, usize)> = self.nodes[nb]
                .neighbors
                .iter()
                .copied()
                .map(|i| {
                    let s = cosine(&self.nodes[i].row.vector, &nb_vec).unwrap_or(f32::MIN);
                    (s, i)
                })
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
            scored.truncate(self.m);
            self.nodes[nb].neighbors = scored.into_iter().map(|(_, i)| i).collect();
        }
    }

    fn search(&self, query: &[f32], k: usize) -> Vec<SearchHit> {
        self.search_candidates(query, k.max(8).max(self.m))
            .into_iter()
            .take(k)
            .map(|(score, idx)| SearchHit {
                id: self.nodes[idx].row.id.clone(),
                score,
                text: self.nodes[idx].row.text.clone(),
            })
            .collect()
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }
}

/// Max-heap by score for candidate expansion.
#[derive(Clone, Copy)]
struct Cand {
    score: f32,
    idx: usize,
}

impl PartialEq for Cand {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.idx == other.idx
    }
}
impl Eq for Cand {}
impl PartialOrd for Cand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cand {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.idx.cmp(&other.idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hnsw_finds_near_neighbor() {
        let mut idx = HnswLite::new(4);
        idx.upsert("a".into(), vec![1.0, 0.0], "alpha".into());
        idx.upsert("b".into(), vec![0.0, 1.0], "beta".into());
        idx.upsert("c".into(), vec![0.95, 0.05], "alphaish".into());
        let hits = idx.search(&[1.0, 0.0], 2);
        assert_eq!(hits[0].id, "a");
        assert!(hits.iter().any(|h| h.id == "c"));
    }
}
