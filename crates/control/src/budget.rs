//! Budget counters: tokens, bytes, wall-time (`budget.exhausted`).

use types::ErrorCode;

/// Hard caps for a principal / binding (unset = unlimited for that dimension).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BudgetLimits {
    pub max_tokens: Option<u64>,
    pub max_bytes: Option<u64>,
    pub max_wall_ms: Option<u64>,
}

/// Accumulated usage against [`BudgetLimits`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BudgetUsage {
    pub tokens: u64,
    pub bytes: u64,
    pub wall_ms: u64,
}

/// Mutable ledger that rejects charges past configured caps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetLedger {
    limits: BudgetLimits,
    usage: BudgetUsage,
}

impl BudgetLedger {
    #[must_use]
    pub fn new(limits: BudgetLimits) -> Self {
        Self {
            limits,
            usage: BudgetUsage::default(),
        }
    }

    /// Unlimited on all dimensions.
    #[must_use]
    pub fn unlimited() -> Self {
        Self::new(BudgetLimits::default())
    }

    #[must_use]
    pub fn limits(&self) -> &BudgetLimits {
        &self.limits
    }

    #[must_use]
    pub fn usage(&self) -> &BudgetUsage {
        &self.usage
    }

    /// Charge token units.
    ///
    /// # Errors
    /// Returns [`ErrorCode::BudgetExhausted`] when the charge would exceed `max_tokens`.
    pub fn charge_tokens(&mut self, n: u64) -> Result<(), ErrorCode> {
        Self::apply_charge(n, self.limits.max_tokens, &mut self.usage.tokens)
    }

    /// Charge byte units.
    ///
    /// # Errors
    /// Returns [`ErrorCode::BudgetExhausted`] when the charge would exceed `max_bytes`.
    pub fn charge_bytes(&mut self, n: u64) -> Result<(), ErrorCode> {
        Self::apply_charge(n, self.limits.max_bytes, &mut self.usage.bytes)
    }

    /// Charge wall-clock milliseconds.
    ///
    /// # Errors
    /// Returns [`ErrorCode::BudgetExhausted`] when the charge would exceed `max_wall_ms`.
    pub fn charge_wall_ms(&mut self, n: u64) -> Result<(), ErrorCode> {
        Self::apply_charge(n, self.limits.max_wall_ms, &mut self.usage.wall_ms)
    }

    fn apply_charge(n: u64, max: Option<u64>, used: &mut u64) -> Result<(), ErrorCode> {
        if let Some(cap) = max {
            let next = used.saturating_add(n);
            if next > cap {
                return Err(ErrorCode::BudgetExhausted);
            }
            *used = next;
        } else {
            *used = used.saturating_add(n);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_accepts_large_charges() {
        let mut ledger = BudgetLedger::unlimited();
        ledger.charge_tokens(1_000_000).expect("tokens");
        ledger.charge_bytes(1_000_000).expect("bytes");
        ledger.charge_wall_ms(1_000_000).expect("wall");
        assert_eq!(ledger.usage().tokens, 1_000_000);
    }

    #[test]
    fn token_cap_exhausts() {
        let mut ledger = BudgetLedger::new(BudgetLimits {
            max_tokens: Some(10),
            max_bytes: None,
            max_wall_ms: None,
        });
        ledger.charge_tokens(7).expect("ok");
        assert_eq!(ledger.charge_tokens(4), Err(ErrorCode::BudgetExhausted));
        assert_eq!(ledger.usage().tokens, 7);
        ledger.charge_tokens(3).expect("exact remaining");
        assert_eq!(ledger.usage().tokens, 10);
        assert_eq!(ledger.charge_tokens(1), Err(ErrorCode::BudgetExhausted));
    }

    #[test]
    fn bytes_and_wall_caps_independent() {
        let mut ledger = BudgetLedger::new(BudgetLimits {
            max_tokens: None,
            max_bytes: Some(100),
            max_wall_ms: Some(50),
        });
        ledger.charge_bytes(100).expect("bytes ok");
        assert_eq!(ledger.charge_bytes(1), Err(ErrorCode::BudgetExhausted));
        ledger.charge_wall_ms(50).expect("wall ok");
        assert_eq!(ledger.charge_wall_ms(1), Err(ErrorCode::BudgetExhausted));
        ledger.charge_tokens(999).expect("tokens unlimited");
    }
}
