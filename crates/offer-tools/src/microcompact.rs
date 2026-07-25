//! Keep only the last N tool results (microcompact for context windows).

use crate::loop_types::ToolResult;

/// Retains the newest `keep` tool results; older ones are dropped.
#[derive(Clone, Debug)]
pub struct Microcompact {
    keep: usize,
}

impl Microcompact {
    #[must_use]
    pub fn new(keep: usize) -> Self {
        Self { keep: keep.max(1) }
    }

    /// Compact in place (oldest first in the vec).
    pub fn compact(&self, results: &mut Vec<ToolResult>) {
        if results.len() <= self.keep {
            return;
        }
        let drop_n = results.len() - self.keep;
        results.drain(0..drop_n);
    }

    /// Return a compacted copy.
    #[must_use]
    pub fn apply(&self, results: &[ToolResult]) -> Vec<ToolResult> {
        let mut v = results.to_vec();
        self.compact(&mut v);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tr(id: &str) -> ToolResult {
        ToolResult {
            call_id: id.into(),
            tool: "echo".into(),
            ok: true,
            output: json!({}),
        }
    }

    #[test]
    fn keeps_last_n() {
        let mut v = vec![tr("1"), tr("2"), tr("3"), tr("4")];
        Microcompact::new(2).compact(&mut v);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].call_id, "3");
        assert_eq!(v[1].call_id, "4");
    }

    #[test]
    fn short_list_unchanged() {
        let mut v = vec![tr("1")];
        Microcompact::new(3).compact(&mut v);
        assert_eq!(v.len(), 1);
    }
}
