//! Principal × offer allow/deny (`policy.denied`).

use std::collections::{HashMap, HashSet};

use types::{ErrorCode, OfferId};

/// Process-local policy: ambient allow-all, or principal allowlists.
#[derive(Clone, Debug, Default)]
pub struct PolicyEngine {
    /// `None` = ambient trust (allow all). `Some` = deny unless granted.
    rules: Option<HashMap<String, HashSet<String>>>,
}

impl PolicyEngine {
    /// Local / stdio ambient trust — every principal may use every offer.
    #[must_use]
    pub fn ambient() -> Self {
        Self { rules: None }
    }

    /// Empty allowlist — deny until [`Self::grant`].
    #[must_use]
    pub fn allowlist() -> Self {
        Self {
            rules: Some(HashMap::new()),
        }
    }

    /// Permit `principal` to bind/invoke `offer_id`.
    pub fn grant(&mut self, principal: impl Into<String>, offer_id: &OfferId) {
        let rules = self.rules.get_or_insert_with(HashMap::new);
        rules
            .entry(principal.into())
            .or_default()
            .insert(offer_id.as_str().to_owned());
    }

    /// Check whether `principal` may use `offer_id`.
    ///
    /// # Errors
    /// Returns [`ErrorCode::PolicyDenied`] when the allowlist rejects the pair.
    pub fn check(&self, principal: &str, offer_id: &OfferId) -> Result<(), ErrorCode> {
        let Some(rules) = &self.rules else {
            return Ok(());
        };
        let allowed = rules
            .get(principal)
            .is_some_and(|offers| offers.contains(offer_id.as_str()));
        if allowed {
            Ok(())
        } else {
            Err(ErrorCode::PolicyDenied)
        }
    }

    #[must_use]
    pub fn is_ambient(&self) -> bool {
        self.rules.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambient_allows_any_pair() {
        let engine = PolicyEngine::ambient();
        let offer = OfferId::new("llm.chat").expect("valid");
        engine.check("anyone", &offer).expect("allowed");
        assert!(engine.is_ambient());
    }

    #[test]
    fn allowlist_denies_until_granted() {
        let mut engine = PolicyEngine::allowlist();
        let offer = OfferId::new("llm.chat").expect("valid");
        assert_eq!(engine.check("alice", &offer), Err(ErrorCode::PolicyDenied));

        engine.grant("alice", &offer);
        engine.check("alice", &offer).expect("granted");
        assert_eq!(engine.check("bob", &offer), Err(ErrorCode::PolicyDenied));
    }

    #[test]
    fn grant_is_offer_specific() {
        let mut engine = PolicyEngine::allowlist();
        let chat = OfferId::new("llm.chat").expect("valid");
        let exec = OfferId::new("sandbox.exec").expect("valid");
        engine.grant("alice", &chat);
        assert_eq!(engine.check("alice", &exec), Err(ErrorCode::PolicyDenied));
        engine.check("alice", &chat).expect("chat ok");
    }
}
