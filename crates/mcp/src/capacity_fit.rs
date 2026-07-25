//! Capacity-backed [`FitAdvisor`] for MCP (composes offers without crate edge).

use offer_capacity::{probe_from_env, rank_models, GovernorBudget, HardwareProbe, ModelCandidate};
use offer_llm::{FitAdvisor, PreflightCandidate};
use serde_json::{json, Value};

/// Ranks via `offer_capacity::rank_models` against a live or fake probe.
pub struct CapacityFitAdvisor {
    probe: Box<dyn HardwareProbe>,
    budget: GovernorBudget,
}

impl CapacityFitAdvisor {
    #[must_use]
    pub fn from_env() -> Self {
        let probe = probe_from_env();
        Self {
            probe,
            budget: GovernorBudget::default(),
        }
    }
}

impl FitAdvisor for CapacityFitAdvisor {
    fn rank(&self, candidates: &[PreflightCandidate]) -> Vec<Value> {
        let snap = self.probe.probe().expect("probe snapshot");
        let models: Vec<ModelCandidate> = candidates
            .iter()
            .map(|c| ModelCandidate {
                id: c.id.clone(),
                ram_mb: c.ram_mb,
                vram_mb: c.vram_mb,
            })
            .collect();
        rank_models(&snap, &self.budget, &models)
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "score": r.score,
                    "fits": r.fits,
                    "reason": r.reason,
                })
            })
            .collect()
    }
}
