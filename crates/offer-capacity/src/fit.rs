//! Model fit ranking for preflight/scheduling (`sak273-a`).

use serde::{Deserialize, Serialize};

use crate::governor::GovernorBudget;
use crate::probe::HardwareSnapshot;

/// Candidate model / workload for fit ranking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelCandidate {
    pub id: String,
    /// Estimated RAM needed (MB).
    pub ram_mb: u64,
    /// Estimated VRAM needed (MB); 0 = CPU-only.
    #[serde(default)]
    pub vram_mb: u64,
}

/// Ranked fit result (higher score = better).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FitRank {
    pub id: String,
    pub score: f32,
    pub fits: bool,
    pub reason: String,
}

/// Rank candidates by fit against snapshot + budget.
///
/// Score ≈ free headroom after claim; non-fitting candidates sort last.
#[must_use]
pub fn rank_models(
    snap: &HardwareSnapshot,
    budget: &GovernorBudget,
    candidates: &[ModelCandidate],
) -> Vec<FitRank> {
    let mut ranks: Vec<FitRank> = candidates
        .iter()
        .map(|c| {
            let mut fits = true;
            let mut reason = "ok".to_owned();
            if c.ram_mb > snap.available_ram_mb {
                fits = false;
                reason = format!(
                    "ram_mb {} > available_ram_mb {}",
                    c.ram_mb, snap.available_ram_mb
                );
            } else if budget.max_ram_mb > 0 && c.ram_mb > budget.max_ram_mb {
                fits = false;
                reason = format!("ram_mb {} > max_ram_mb {}", c.ram_mb, budget.max_ram_mb);
            } else if c.vram_mb > 0 && c.vram_mb > snap.available_vram_mb {
                fits = false;
                reason = format!(
                    "vram_mb {} > available_vram_mb {}",
                    c.vram_mb, snap.available_vram_mb
                );
            } else if budget.max_vram_mb > 0 && c.vram_mb > budget.max_vram_mb {
                fits = false;
                reason = format!("vram_mb {} > max_vram_mb {}", c.vram_mb, budget.max_vram_mb);
            }

            let headroom = snap.available_ram_mb.saturating_sub(c.ram_mb) as f32;
            let score = if fits {
                headroom / (snap.total_ram_mb.max(1) as f32)
            } else {
                -1.0
            };
            FitRank {
                id: c.id.clone(),
                score,
                fits,
                reason,
            }
        })
        .collect();

    ranks.sort_by(|a, b| {
        b.fits
            .cmp(&a.fits)
            .then(
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.id.cmp(&b.id))
    });
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::HardwareSnapshot;

    #[test]
    fn ranks_smaller_first_when_both_fit() {
        let snap = HardwareSnapshot {
            total_ram_mb: 16_384,
            available_ram_mb: 8_192,
            cpu_logical: 8,
            cpu_usage_pct: 10.0,
            total_vram_mb: 8_192,
            available_vram_mb: 8_192,
            source: "t".into(),
        };
        let budget = GovernorBudget::default();
        let ranks = rank_models(
            &snap,
            &budget,
            &[
                ModelCandidate {
                    id: "big".into(),
                    ram_mb: 6_000,
                    vram_mb: 0,
                },
                ModelCandidate {
                    id: "small".into(),
                    ram_mb: 1_000,
                    vram_mb: 0,
                },
            ],
        );
        assert_eq!(ranks[0].id, "small");
        assert!(ranks[0].fits && ranks[1].fits);
    }

    #[test]
    fn non_fitting_sorts_last() {
        let snap = HardwareSnapshot {
            total_ram_mb: 4_096,
            available_ram_mb: 2_000,
            cpu_logical: 4,
            cpu_usage_pct: 5.0,
            total_vram_mb: 0,
            available_vram_mb: 0,
            source: "t".into(),
        };
        let ranks = rank_models(
            &snap,
            &GovernorBudget::default(),
            &[
                ModelCandidate {
                    id: "huge".into(),
                    ram_mb: 99_000,
                    vram_mb: 0,
                },
                ModelCandidate {
                    id: "ok".into(),
                    ram_mb: 500,
                    vram_mb: 0,
                },
            ],
        );
        assert_eq!(ranks[0].id, "ok");
        assert!(!ranks[1].fits);
    }
}
