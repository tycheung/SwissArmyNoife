//! SSH fleet capacity probe stub (`sak275` — stub complete).
//!
//! Enterprise / curated tier: aggregate remote host signals over SSH.
//! Select with `CAPACITY_PROBE=ssh` via [`probe_from_env`](crate::probe::probe_from_env).
//!
//! **OSS behavior:** no SSH sessions are opened. [`HardwareProbe::probe`] always returns
//! [`ErrorCode::ProviderUnreachable`]. Real fleet probing is an enterprise follow-up;
//! see `offer-capacity/README.md`.
//!
//! Offline `/proc/meminfo` → [`HardwareSnapshot`] helpers live in [`crate::ssh_meminfo`]
//! (`sak275-l`). Live SSH (`CAPACITY_SSH_HOSTS`) remains TODO.

use serde_json::{json, Value};
use types::ErrorCode;

use crate::probe::{HardwareProbe, HardwareSnapshot};
use crate::ssh_meminfo::hardware_snapshot_from_meminfo;

/// Placeholder for SSH-backed fleet probes (enterprise / curated).
#[derive(Clone, Debug, Default)]
pub struct SshFleetProbe;

impl SshFleetProbe {
    /// Wire JSON shape when reporting stub status without an error envelope.
    #[must_use]
    pub fn stub_payload() -> Value {
        json!({ "ok": false, "reason": "ssh_probe_stub" })
    }

    /// Build from `CAPACITY_PROBE=ssh` (see [`probe_from_env`](crate::probe::probe_from_env)).
    #[must_use]
    pub fn from_env() -> Box<dyn HardwareProbe> {
        Box::new(Self)
    }

    /// Map a remote `/proc/meminfo` dump (fixture or future SSH stdout) into a snapshot.
    ///
    /// Does **not** open SSH. Used by unit tests today; live fleet path will call this
    /// after a remote `cat /proc/meminfo` (TODO: `CAPACITY_SSH_HOSTS`).
    ///
    /// # Errors
    /// [`ErrorCode::ProviderUnreachable`] when meminfo cannot be parsed.
    pub fn snapshot_from_meminfo(
        meminfo: &str,
        host_id: &str,
        cpu_logical: u32,
        cpu_usage_pct: f32,
    ) -> Result<HardwareSnapshot, ErrorCode> {
        hardware_snapshot_from_meminfo(
            meminfo,
            format!("ssh:{host_id}"),
            cpu_logical,
            cpu_usage_pct,
        )
    }
}

impl HardwareProbe for SshFleetProbe {
    fn probe(&self) -> Result<HardwareSnapshot, ErrorCode> {
        // TODO(sak275): when CAPACITY_SSH_HOSTS is set, SSH + cat /proc/meminfo then
        // `snapshot_from_meminfo`. OSS default stays unreachable (no sockets).
        Err(ErrorCode::ProviderUnreachable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_fleet_probe_returns_unreachable() {
        assert_eq!(SshFleetProbe.probe(), Err(ErrorCode::ProviderUnreachable));
    }

    #[test]
    fn ssh_fleet_probe_stub_payload_shape() {
        let v = SshFleetProbe::stub_payload();
        assert_eq!(v["ok"], false);
        assert_eq!(v["reason"], "ssh_probe_stub");
    }

    #[test]
    fn ssh_fleet_probe_from_env_box() {
        let prior = std::env::var("CAPACITY_PROBE").ok();
        std::env::set_var("CAPACITY_PROBE", "ssh");
        let probe = SshFleetProbe::from_env();
        assert_eq!(probe.probe(), Err(ErrorCode::ProviderUnreachable));
        match prior {
            Some(v) => std::env::set_var("CAPACITY_PROBE", v),
            None => std::env::remove_var("CAPACITY_PROBE"),
        }
    }

    #[test]
    fn ssh_fleet_probe_maps_meminfo_fixture() {
        let meminfo = "\
MemTotal:        1048576 kB
MemAvailable:     524288 kB
";
        let snap = SshFleetProbe::snapshot_from_meminfo(meminfo, "cpu-b", 4, 0.0).expect("snap");
        assert_eq!(snap.total_ram_mb, 1024);
        assert_eq!(snap.available_ram_mb, 512);
        assert_eq!(snap.source, "ssh:cpu-b");
    }
}
