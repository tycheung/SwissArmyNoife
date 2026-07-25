//! Hardware snapshot + probe trait (`sak270-a` / `sak275-b`).

use serde::{Deserialize, Serialize};
use types::ErrorCode;

/// Point-in-time host capacity signal (MB / cores; VRAM optional).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HardwareSnapshot {
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub cpu_logical: u32,
    pub cpu_usage_pct: f32,
    /// 0 when unknown / no GPU probe.
    #[serde(default)]
    pub total_vram_mb: u64,
    #[serde(default)]
    pub available_vram_mb: u64,
    pub source: String,
}

/// Port for local (or fake) hardware probes.
pub trait HardwareProbe: Send + Sync {
    /// Fresh snapshot; SSH fleet returns [`ErrorCode::ProviderUnreachable`] until wired.
    ///
    /// # Errors
    /// [`ErrorCode::ProviderUnreachable`] when the probe backend is unavailable (e.g. SSH fleet stub).
    fn probe(&self) -> Result<HardwareSnapshot, ErrorCode>;
}

/// Select probe implementation from `CAPACITY_PROBE` (`fake` | `ssh` | default local sysinfo).
#[must_use]
pub fn probe_from_env() -> Box<dyn HardwareProbe> {
    match std::env::var("CAPACITY_PROBE").unwrap_or_default().as_str() {
        "fake" => Box::new(FakeProbe::typical_laptop()),
        "ssh" => Box::new(crate::ssh_probe::SshFleetProbe),
        _ => Box::new(crate::sys_probe::LocalSysProbe),
    }
}

/// Deterministic probe for tests / CI (`sak270-a`).
#[derive(Clone, Debug)]
pub struct FakeProbe {
    pub snapshot: HardwareSnapshot,
}

impl FakeProbe {
    #[must_use]
    pub fn typical_laptop() -> Self {
        Self {
            snapshot: HardwareSnapshot {
                total_ram_mb: 16_384,
                available_ram_mb: 8_192,
                cpu_logical: 8,
                cpu_usage_pct: 25.0,
                total_vram_mb: 8_192,
                available_vram_mb: 6_144,
                source: "fake".into(),
            },
        }
    }
}

impl HardwareProbe for FakeProbe {
    fn probe(&self) -> Result<HardwareSnapshot, ErrorCode> {
        Ok(self.snapshot.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_probe_returns_snapshot() {
        let p = FakeProbe::typical_laptop();
        let s = p.probe().expect("snapshot");
        assert_eq!(s.source, "fake");
        assert!(s.total_ram_mb >= s.available_ram_mb);
    }

    #[test]
    fn probe_from_env_fake_and_ssh() {
        let prior = std::env::var("CAPACITY_PROBE").ok();
        std::env::set_var("CAPACITY_PROBE", "fake");
        let fake = probe_from_env();
        assert_eq!(fake.probe().expect("fake").source, "fake");
        std::env::set_var("CAPACITY_PROBE", "ssh");
        let ssh = probe_from_env();
        assert_eq!(ssh.probe(), Err(ErrorCode::ProviderUnreachable));
        match prior {
            Some(v) => std::env::set_var("CAPACITY_PROBE", v),
            None => std::env::remove_var("CAPACITY_PROBE"),
        }
    }
}
