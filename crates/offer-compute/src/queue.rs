//! Work-unit types + [`WorkQueue`] port (`sak291` / `sak292`).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use types::ErrorCode;
use uuid::Uuid;

use crate::merge::MergeHook;
use crate::node::NodeId;

/// Work unit identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkId(Uuid);

impl WorkId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for WorkId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lifecycle of a work unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    Queued,
    Claimed,
    Completed,
    Failed,
}

impl WorkStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    /// # Errors
    /// Unknown status string.
    pub fn parse(s: &str) -> Result<Self, ErrorCode> {
        match s {
            "queued" => Ok(Self::Queued),
            "claimed" => Ok(Self::Claimed),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(ErrorCode::SchemaInvalid),
        }
    }
}

/// Host-authored work item (payload is sanitized on enqueue).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkUnit {
    pub id: WorkId,
    pub kind: String,
    pub payload: Value,
    pub status: WorkStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

/// Port for memory / `SQLite` / Redis queues (`refactor:work-queue-trait`).
pub trait WorkQueue: Send + Sync {
    /// Enqueue after sanitizing payload.
    ///
    /// # Errors
    /// Queue lock or persistence failures.
    fn enqueue(&self, kind: &str, payload: Value) -> Result<WorkUnit, ErrorCode>;

    /// Claim oldest queued unit.
    ///
    /// # Errors
    /// Empty queue (`OfferNotFound`) or store failure.
    fn claim(&self, node: NodeId) -> Result<WorkUnit, ErrorCode>;

    /// Complete a claimed unit via merge hook.
    ///
    /// # Errors
    /// Wrong claimer / status (`PolicyDenied`) or store failure.
    fn complete(
        &self,
        work_id: WorkId,
        node: NodeId,
        result: Value,
        merge: &dyn MergeHook,
    ) -> Result<WorkUnit, ErrorCode>;

    /// Fetch one unit.
    ///
    /// # Errors
    /// Missing unit or store failure.
    fn get(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode>;

    /// List newest-first (cap `limit`).
    ///
    /// # Errors
    /// Queue lock or persistence failures.
    fn list(&self, limit: usize) -> Result<Vec<WorkUnit>, ErrorCode>;

    /// Reset a claimed (or failed) unit back to queued (`sak428-c`).
    ///
    /// # Errors
    /// Missing unit, invalid status, or store failure.
    fn requeue(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode>;
}
