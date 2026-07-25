//! Local `sysinfo` hardware probe (`sak270-b`).

use sysinfo::System;
use types::ErrorCode;

use crate::probe::{HardwareProbe, HardwareSnapshot};

/// Probe using the host OS via `sysinfo` (RAM + CPU; VRAM left 0).
#[derive(Clone, Debug, Default)]
pub struct LocalSysProbe;

impl HardwareProbe for LocalSysProbe {
    fn probe(&self) -> Result<HardwareSnapshot, ErrorCode> {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_usage();
        // First refresh often reports 0 usage; second pass settles.
        std::thread::sleep(std::time::Duration::from_millis(20));
        sys.refresh_cpu_usage();

        let total_ram_mb = sys.total_memory() / (1024 * 1024);
        let available_ram_mb = sys.available_memory() / (1024 * 1024);
        let cpu_logical = sys.cpus().len() as u32;
        let cpu_usage_pct = sys.global_cpu_usage();

        Ok(HardwareSnapshot {
            total_ram_mb,
            available_ram_mb,
            cpu_logical: cpu_logical.max(1),
            cpu_usage_pct,
            total_vram_mb: 0,
            available_vram_mb: 0,
            source: "sysinfo".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_probe_non_zero_ram() {
        let s = LocalSysProbe.probe().expect("snapshot");
        assert_eq!(s.source, "sysinfo");
        assert!(s.total_ram_mb > 0, "total_ram_mb={}", s.total_ram_mb);
        assert!(s.cpu_logical >= 1);
    }
}
