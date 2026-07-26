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

    /// Prometheus text exposition (`sak528-d` / sak066 deepen).
    #[must_use]
    pub fn to_prometheus(&self) -> String {
        format!(
            "# HELP sak_invokes_total Total invokes recorded by the admin meter.\n\
             # TYPE sak_invokes_total counter\n\
             sak_invokes_total {}\n\
             # HELP sak_bindings_live Live binding count.\n\
             # TYPE sak_bindings_live gauge\n\
             sak_bindings_live {}\n\
             # HELP sak_offers_catalog Catalogued offer count.\n\
             # TYPE sak_offers_catalog gauge\n\
             sak_offers_catalog {}\n",
            self.invoke_count, self.binding_count, self.offer_count
        )
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

    #[test]
    fn prometheus_text_has_typed_metrics() {
        let snap = MeterSnapshot::new(3, 2, 10);
        let text = snap.to_prometheus();
        assert!(text.contains("# TYPE sak_invokes_total counter"));
        assert!(text.contains("sak_invokes_total 3"));
        assert!(text.contains("# TYPE sak_bindings_live gauge"));
        assert!(text.contains("sak_bindings_live 2"));
        assert!(text.contains("sak_offers_catalog 10"));
        assert!(text.ends_with('\n'));
    }
}
