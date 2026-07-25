//! Binding risk caps: tool steps, shell invocations, write bytes.

use serde_json::Value;
use types::ErrorCode;

/// Hard caps frozen onto a binding (unset = unlimited for that dimension).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RiskCaps {
    pub max_tool_steps: Option<u64>,
    pub max_shell_invocations: Option<u64>,
    pub max_write_bytes: Option<u64>,
}

impl RiskCaps {
    /// Parse from binding `policy_json` (`risk_caps` object). Missing keys → unlimited.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        let Some(obj) = policy.get("risk_caps").and_then(Value::as_object) else {
            return Self::default();
        };
        Self {
            max_tool_steps: obj.get("max_tool_steps").and_then(Value::as_u64),
            max_shell_invocations: obj.get("max_shell_invocations").and_then(Value::as_u64),
            max_write_bytes: obj.get("max_write_bytes").and_then(Value::as_u64),
        }
    }
}

/// Accumulated risk usage against [`RiskCaps`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RiskUsage {
    pub tool_steps: u64,
    pub shell_invocations: u64,
    pub write_bytes: u64,
}

/// Mutable ledger that rejects charges past binding risk caps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskLedger {
    caps: RiskCaps,
    usage: RiskUsage,
}

impl RiskLedger {
    #[must_use]
    pub fn new(caps: RiskCaps) -> Self {
        Self {
            caps,
            usage: RiskUsage::default(),
        }
    }

    /// Unlimited on all dimensions.
    #[must_use]
    pub fn unlimited() -> Self {
        Self::new(RiskCaps::default())
    }

    /// Build from a binding policy snapshot.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        Self::new(RiskCaps::from_policy(policy))
    }

    #[must_use]
    pub fn caps(&self) -> &RiskCaps {
        &self.caps
    }

    #[must_use]
    pub fn usage(&self) -> &RiskUsage {
        &self.usage
    }

    /// Charge one tool-loop step.
    ///
    /// # Errors
    /// Returns [`ErrorCode::BudgetExhausted`] when past `max_tool_steps`.
    pub fn charge_tool_step(&mut self) -> Result<(), ErrorCode> {
        Self::apply_charge(1, self.caps.max_tool_steps, &mut self.usage.tool_steps)
    }

    /// Charge one shell / sandbox.exec invocation.
    ///
    /// # Errors
    /// Returns [`ErrorCode::BudgetExhausted`] when past `max_shell_invocations`.
    pub fn charge_shell(&mut self) -> Result<(), ErrorCode> {
        Self::apply_charge(
            1,
            self.caps.max_shell_invocations,
            &mut self.usage.shell_invocations,
        )
    }

    /// Charge filesystem write bytes.
    ///
    /// # Errors
    /// Returns [`ErrorCode::BudgetExhausted`] when past `max_write_bytes`.
    pub fn charge_write_bytes(&mut self, n: u64) -> Result<(), ErrorCode> {
        Self::apply_charge(n, self.caps.max_write_bytes, &mut self.usage.write_bytes)
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
    use serde_json::json;

    #[test]
    fn from_policy_reads_risk_caps() {
        let caps = RiskCaps::from_policy(&json!({
            "risk_caps": {
                "max_tool_steps": 3,
                "max_shell_invocations": 2,
                "max_write_bytes": 100
            }
        }));
        assert_eq!(caps.max_tool_steps, Some(3));
        assert_eq!(caps.max_shell_invocations, Some(2));
        assert_eq!(caps.max_write_bytes, Some(100));
    }

    #[test]
    fn missing_risk_caps_is_unlimited() {
        let caps = RiskCaps::from_policy(&json!({"other": true}));
        assert_eq!(caps, RiskCaps::default());
    }

    #[test]
    fn tool_and_shell_caps_exhaust() {
        let mut ledger = RiskLedger::new(RiskCaps {
            max_tool_steps: Some(2),
            max_shell_invocations: Some(1),
            max_write_bytes: None,
        });
        ledger.charge_tool_step().expect("1");
        ledger.charge_tool_step().expect("2");
        assert_eq!(ledger.charge_tool_step(), Err(ErrorCode::BudgetExhausted));
        ledger.charge_shell().expect("shell");
        assert_eq!(ledger.charge_shell(), Err(ErrorCode::BudgetExhausted));
        ledger.charge_write_bytes(999).expect("writes unlimited");
    }

    #[test]
    fn write_bytes_cap() {
        let mut ledger = RiskLedger::from_policy(&json!({
            "risk_caps": { "max_write_bytes": 10 }
        }));
        ledger.charge_write_bytes(7).expect("ok");
        assert_eq!(
            ledger.charge_write_bytes(4),
            Err(ErrorCode::BudgetExhausted)
        );
        assert_eq!(ledger.usage().write_bytes, 7);
    }
}
