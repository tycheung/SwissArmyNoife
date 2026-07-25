//! Governor budgets from binding policy (`sak271-a`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Soft ceilings for admission / fit ranking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GovernorBudget {
    /// Max RAM the workload may claim (MB).
    pub max_ram_mb: u64,
    /// Max VRAM claim (MB); 0 = ignore VRAM.
    #[serde(default)]
    pub max_vram_mb: u64,
    /// Deny when host CPU usage is at or above this percent.
    #[serde(default = "default_max_cpu")]
    pub max_cpu_pct: f32,
    /// Require at least this much free RAM (MB) after claim.
    #[serde(default)]
    pub min_free_ram_mb: u64,
}

fn default_max_cpu() -> f32 {
    95.0
}

impl Default for GovernorBudget {
    fn default() -> Self {
        Self {
            max_ram_mb: 8_192,
            max_vram_mb: 0,
            max_cpu_pct: 95.0,
            min_free_ram_mb: 512,
        }
    }
}

impl GovernorBudget {
    /// Parse from frozen binding policy JSON.
    ///
    /// Accepts `{ "capacity": { ... } }` or a bare budget object.
    #[must_use]
    pub fn from_policy(params: &Value) -> Self {
        let node = params
            .get("capacity")
            .or_else(|| params.get("governor"))
            .unwrap_or(params);
        serde_json::from_value(node.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn from_policy_nested() {
        let b = GovernorBudget::from_policy(&json!({
            "capacity": { "max_ram_mb": 4096, "max_cpu_pct": 80.0 }
        }));
        assert_eq!(b.max_ram_mb, 4096);
        assert!((b.max_cpu_pct - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn from_policy_defaults() {
        let b = GovernorBudget::from_policy(&json!({}));
        assert_eq!(b, GovernorBudget::default());
    }
}
