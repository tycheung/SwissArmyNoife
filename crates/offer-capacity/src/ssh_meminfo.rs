//! Parse remote `/proc/meminfo` into capacity fields (`sak275-l`).
//!
//! Offline helpers for a future SSH fleet probe. No SSH sockets — unit tests use
//! fixture strings only. Live remote execution remains deferred.

use types::ErrorCode;

use crate::probe::HardwareSnapshot;

/// `MemTotal` / `MemAvailable` values from `/proc/meminfo` (kibibytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeminfoKb {
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
}

/// Extract `MemTotal` and `MemAvailable` (kB) from a `/proc/meminfo` dump.
///
/// Falls back to `MemFree` + `Buffers` + `Cached` when `MemAvailable` is absent
/// (older kernels). Returns [`None`] when `MemTotal` is missing or zero.
#[must_use]
pub fn parse_meminfo_kb(text: &str) -> Option<MeminfoKb> {
    let mut mem_total_kb = None;
    let mut mem_available_kb = None;
    let mut mem_free_kb = 0_u64;
    let mut buffers_kb = 0_u64;
    let mut cached_kb = 0_u64;

    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(val) = parts.next().and_then(|v| v.parse::<u64>().ok()) else {
            continue;
        };
        match key.trim_end_matches(':') {
            "MemTotal" => mem_total_kb = Some(val),
            "MemAvailable" => mem_available_kb = Some(val),
            "MemFree" => mem_free_kb = val,
            "Buffers" => buffers_kb = val,
            "Cached" => cached_kb = val,
            _ => {}
        }
    }

    let mem_total_kb = mem_total_kb.filter(|&v| v > 0)?;
    let mem_available_kb = mem_available_kb.unwrap_or_else(|| {
        mem_free_kb
            .saturating_add(buffers_kb)
            .saturating_add(cached_kb)
    });
    Some(MeminfoKb {
        mem_total_kb,
        mem_available_kb: mem_available_kb.min(mem_total_kb),
    })
}

fn kb_to_mb(kb: u64) -> u64 {
    kb / 1024
}

/// Map a `/proc/meminfo` dump into a [`HardwareSnapshot`].
///
/// CPU fields are supplied by the caller (remote `nproc` / loadavg still TODO for live SSH).
/// VRAM fields are zero until a GPU query path exists.
///
/// # Errors
/// [`ErrorCode::ProviderUnreachable`] when meminfo cannot be parsed.
pub fn hardware_snapshot_from_meminfo(
    text: &str,
    source: impl Into<String>,
    cpu_logical: u32,
    cpu_usage_pct: f32,
) -> Result<HardwareSnapshot, ErrorCode> {
    let info = parse_meminfo_kb(text).ok_or(ErrorCode::ProviderUnreachable)?;
    Ok(HardwareSnapshot {
        total_ram_mb: kb_to_mb(info.mem_total_kb),
        available_ram_mb: kb_to_mb(info.mem_available_kb),
        cpu_logical,
        cpu_usage_pct,
        total_vram_mb: 0,
        available_vram_mb: 0,
        source: source.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
MemTotal:       16384000 kB
MemFree:         2048000 kB
MemAvailable:    8192000 kB
Buffers:          512000 kB
Cached:          4096000 kB
SwapTotal:             0 kB
";

    const FIXTURE_NO_AVAILABLE: &str = "\
MemTotal:        2097152 kB
MemFree:          524288 kB
Buffers:          131072 kB
Cached:           262144 kB
";

    #[test]
    fn parse_meminfo_reads_total_and_available() {
        let info = parse_meminfo_kb(FIXTURE).expect("parse");
        assert_eq!(info.mem_total_kb, 16_384_000);
        assert_eq!(info.mem_available_kb, 8_192_000);
    }

    #[test]
    fn parse_meminfo_falls_back_without_available() {
        let info = parse_meminfo_kb(FIXTURE_NO_AVAILABLE).expect("parse");
        assert_eq!(info.mem_total_kb, 2_097_152);
        assert_eq!(info.mem_available_kb, 524_288 + 131_072 + 262_144);
    }

    #[test]
    fn parse_meminfo_rejects_empty() {
        assert!(parse_meminfo_kb("").is_none());
        assert!(parse_meminfo_kb("SwapTotal: 0 kB\n").is_none());
    }

    #[test]
    fn snapshot_from_fixture_maps_mb_and_source() {
        let snap = hardware_snapshot_from_meminfo(FIXTURE, "ssh:gpu-a", 8, 12.5).expect("snap");
        assert_eq!(snap.total_ram_mb, 16_000);
        assert_eq!(snap.available_ram_mb, 8_000);
        assert_eq!(snap.cpu_logical, 8);
        assert!((snap.cpu_usage_pct - 12.5).abs() < f32::EPSILON);
        assert_eq!(snap.total_vram_mb, 0);
        assert_eq!(snap.source, "ssh:gpu-a");
    }
}
