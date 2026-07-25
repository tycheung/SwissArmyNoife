//! Simple meter JSONL export (`sak066-a`).

use serde_json::json;

/// Point-in-time broker counters for admin export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeterSnapshot {
    pub invoke_count: u64,
    pub binding_count: u64,
    pub offer_count: u64,
}

impl MeterSnapshot {
    #[must_use]
    pub fn new(invoke_count: u64, binding_count: u64, offer_count: u64) -> Self {
        Self {
            invoke_count,
            binding_count,
            offer_count,
        }
    }

    /// Newline-delimited JSON metric lines.
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        let lines = [
            json!({"metric": "invokes_total", "value": self.invoke_count}),
            json!({"metric": "bindings_live", "value": self.binding_count}),
            json!({"metric": "offers_catalog", "value": self.offer_count}),
        ];
        lines
            .iter()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonl_has_three_metrics() {
        let snap = MeterSnapshot::new(3, 2, 10);
        let text = snap.to_jsonl();
        assert!(text.contains("\"invokes_total\""));
        assert!(text.contains("\"bindings_live\""));
        assert!(text.contains("\"offers_catalog\""));
        assert_eq!(text.lines().count(), 3);
    }

    #[test]
    fn jsonl_values_match_snapshot() {
        let snap = MeterSnapshot::new(7, 4, 12);
        let text = snap.to_jsonl();
        assert!(text.contains("\"value\":7"));
        assert!(text.contains("\"value\":4"));
        assert!(text.contains("\"value\":12"));
        assert!(text.ends_with('\n'));
    }
}
