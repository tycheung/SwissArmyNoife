//! Pressure sampling vs governor (`sak272-a`).

use serde::{Deserialize, Serialize};
use types::ErrorCode;

use crate::governor::GovernorBudget;
use crate::probe::HardwareSnapshot;

/// Probe vs budget ratios for admission control.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureSample {
    pub ram_used_ratio: f32,
    pub cpu_usage_pct: f32,
    pub vram_used_ratio: Option<f32>,
    pub available_ram_mb: u64,
    pub admit: bool,
    pub reason: String,
}

/// Compute pressure and admission decision.
#[must_use]
pub fn sample_pressure(snap: &HardwareSnapshot, budget: &GovernorBudget) -> PressureSample {
    let ram_used = snap.total_ram_mb.saturating_sub(snap.available_ram_mb);
    let ram_used_ratio = if snap.total_ram_mb == 0 {
        1.0
    } else {
        mb_f32(ram_used) / mb_f32(snap.total_ram_mb)
    };

    let vram_used_ratio = if snap.total_vram_mb > 0 {
        let used = snap.total_vram_mb.saturating_sub(snap.available_vram_mb);
        Some(mb_f32(used) / mb_f32(snap.total_vram_mb))
    } else {
        None
    };

    let mut admit = true;
    let mut reason = "ok".to_owned();

    if snap.available_ram_mb < budget.min_free_ram_mb {
        admit = false;
        reason = format!(
            "available_ram_mb {} < min_free_ram_mb {}",
            snap.available_ram_mb, budget.min_free_ram_mb
        );
    } else if snap.cpu_usage_pct >= budget.max_cpu_pct {
        admit = false;
        reason = format!(
            "cpu_usage_pct {} >= max_cpu_pct {}",
            snap.cpu_usage_pct, budget.max_cpu_pct
        );
    } else if budget.max_ram_mb > 0 && budget.max_ram_mb > snap.available_ram_mb {
        admit = false;
        reason = format!(
            "max_ram_mb {} > available_ram_mb {}",
            budget.max_ram_mb, snap.available_ram_mb
        );
    } else if budget.max_vram_mb > 0 && budget.max_vram_mb > snap.available_vram_mb {
        admit = false;
        reason = format!(
            "max_vram_mb {} > available_vram_mb {}",
            budget.max_vram_mb, snap.available_vram_mb
        );
    }

    PressureSample {
        ram_used_ratio,
        cpu_usage_pct: snap.cpu_usage_pct,
        vram_used_ratio,
        available_ram_mb: snap.available_ram_mb,
        admit,
        reason,
    }
}

/// Map deny → [`ErrorCode::BudgetExhausted`].
///
/// # Errors
/// Returns [`ErrorCode::BudgetExhausted`] when `sample.admit` is false.
pub fn admit_or_err(sample: &PressureSample) -> Result<(), ErrorCode> {
    if sample.admit {
        Ok(())
    } else {
        Err(ErrorCode::BudgetExhausted)
    }
}

/// MB counts for ratio scoring stay far below `f32` precision limits in practice.
#[allow(clippy::cast_precision_loss)]
#[inline]
fn mb_f32(v: u64) -> f32 {
    v as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::HardwareSnapshot;

    fn snap(avail_ram: u64, cpu: f32) -> HardwareSnapshot {
        HardwareSnapshot {
            total_ram_mb: 16_384,
            available_ram_mb: avail_ram,
            cpu_logical: 8,
            cpu_usage_pct: cpu,
            total_vram_mb: 0,
            available_vram_mb: 0,
            source: "test".into(),
        }
    }

    #[test]
    fn admits_under_budget() {
        let budget = GovernorBudget {
            max_ram_mb: 4_096,
            max_vram_mb: 0,
            max_cpu_pct: 90.0,
            min_free_ram_mb: 512,
        };
        let p = sample_pressure(&snap(8_000, 20.0), &budget);
        assert!(p.admit);
        assert!(admit_or_err(&p).is_ok());
    }

    #[test]
    fn denies_low_free_ram() {
        let budget = GovernorBudget {
            max_ram_mb: 4_096,
            min_free_ram_mb: 2_000,
            ..GovernorBudget::default()
        };
        let p = sample_pressure(&snap(500, 10.0), &budget);
        assert!(!p.admit);
        assert_eq!(admit_or_err(&p), Err(ErrorCode::BudgetExhausted));
    }
}
