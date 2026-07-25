//! SwissArmyNoife `compute.*` mesh (nodes + work queue).

mod memory_queue;
mod merge;
mod node;
mod node_offer;
mod plane;
mod queue;
mod redis_queue;
mod sanitize;
mod sqlite_nodes;
mod sqlite_queue;
mod work_offer;

pub use memory_queue::MemoryQueue;
pub use merge::{IdentityMerge, MergeHook, PreferWorkerMerge};
pub use node::{NodeId, NodeRecord, NodeRegistry, NodeStore};
pub use node_offer::ComputeNodeOffer;
pub use plane::ComputePlane;
pub use queue::{WorkId, WorkQueue, WorkStatus, WorkUnit};
pub use redis_queue::{FakeRedis, RedisBackend, RedisQueue};
pub use sanitize::sanitize_payload;
pub use sqlite_nodes::SqliteNodeRegistry;
pub use sqlite_queue::SqliteQueue;
pub use work_offer::ComputeWorkOffer;
