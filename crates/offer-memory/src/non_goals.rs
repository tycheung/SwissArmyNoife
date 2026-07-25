//! Explicit non-goals for memory/research (sak246).
//!
//! Broker memory + research **do not** implement Maker approve/reject theater UX.
//! Nimbusware (or another harness) projects broker artifacts into operator workflows.
//! `SwissArmyNoife` persists briefs/indexes and enforces egress — it does not host
//! human-in-the-loop approval surfaces inside the OSS broker v0.

#[cfg(test)]
mod tests {
    #[test]
    fn documents_non_goal() {
        let text = include_str!("non_goals.rs");
        assert!(text.contains("Maker approve"));
        assert!(text.contains("does not"));
    }
}
