//! SwissArmyNoife `capacity.*` helpers and offers (Phase 7).

mod fit;
mod fit_offer;
mod governor;
mod pressure;
mod pressure_offer;
mod probe;
mod probe_offer;
mod ssh_inventory;
mod ssh_meminfo;
mod ssh_probe;
mod sys_probe;

pub use fit::{rank_models, FitRank, ModelCandidate};
pub use fit_offer::CapacityFitOffer;
pub use governor::GovernorBudget;
pub use pressure::{admit_or_err, sample_pressure, PressureSample};
pub use pressure_offer::CapacityPressureOffer;
pub use probe::{probe_from_env, FakeProbe, HardwareProbe, HardwareSnapshot};
pub use probe_offer::CapacityProbeOffer;
pub use ssh_inventory::{
    count_hosts_in_inventory_sketch, host_ids_from_inventory_path, host_ids_from_inventory_sketch,
    inventory_path_from_env, unique_host_ids, unique_host_ids_from_env,
    unique_host_ids_from_inventory_path, InventoryLoadError, SSH_INVENTORY_ENV,
};
pub use ssh_meminfo::{hardware_snapshot_from_meminfo, parse_meminfo_kb, MeminfoKb};
pub use ssh_probe::SshFleetProbe;
pub use sys_probe::LocalSysProbe;
