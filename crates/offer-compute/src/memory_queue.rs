//! In-memory FIFO work queue.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use types::ErrorCode;

use crate::merge::MergeHook;
use crate::node::NodeId;
use crate::queue::{WorkId, WorkQueue, WorkStatus, WorkUnit};
use crate::sanitize::sanitize_payload;

/// In-memory FIFO work queue.
#[derive(Debug, Default)]
pub struct MemoryQueue {
    units: Mutex<HashMap<WorkId, WorkUnit>>,
    order: Mutex<Vec<WorkId>>,
}

impl MemoryQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl WorkQueue for MemoryQueue {
    fn enqueue(&self, kind: &str, payload: Value) -> Result<WorkUnit, ErrorCode> {
        let unit = WorkUnit {
            id: WorkId::new(),
            kind: kind.to_owned(),
            payload: sanitize_payload(payload),
            status: WorkStatus::Queued,
            claimed_by: None,
            result: None,
        };
        let mut units = self.units.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let mut order = self.order.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        units.insert(unit.id, unit.clone());
        order.push(unit.id);
        Ok(unit)
    }

    fn claim(&self, node: NodeId) -> Result<WorkUnit, ErrorCode> {
        let mut units = self.units.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let order = self.order.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        for id in order.iter() {
            if let Some(u) = units.get_mut(id) {
                if u.status == WorkStatus::Queued {
                    u.status = WorkStatus::Claimed;
                    u.claimed_by = Some(node);
                    return Ok(u.clone());
                }
            }
        }
        Err(ErrorCode::OfferNotFound)
    }

    fn complete(
        &self,
        work_id: WorkId,
        node: NodeId,
        result: Value,
        merge: &dyn MergeHook,
    ) -> Result<WorkUnit, ErrorCode> {
        let mut units = self.units.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let u = units.get_mut(&work_id).ok_or(ErrorCode::OfferNotFound)?;
        if u.status != WorkStatus::Claimed || u.claimed_by != Some(node) {
            return Err(ErrorCode::PolicyDenied);
        }
        let merged = merge.merge(&u.payload, &result)?;
        u.result = Some(sanitize_payload(merged));
        u.status = WorkStatus::Completed;
        Ok(u.clone())
    }

    fn get(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode> {
        let units = self.units.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        units.get(&work_id).cloned().ok_or(ErrorCode::OfferNotFound)
    }

    fn list(&self, limit: usize) -> Result<Vec<WorkUnit>, ErrorCode> {
        let units = self.units.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let order = self.order.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(order
            .iter()
            .rev()
            .filter_map(|id| units.get(id).cloned())
            .take(limit)
            .collect())
    }

    fn requeue(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode> {
        let mut units = self.units.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let u = units.get_mut(&work_id).ok_or(ErrorCode::OfferNotFound)?;
        if u.status != WorkStatus::Claimed && u.status != WorkStatus::Failed {
            return Err(ErrorCode::PolicyDenied);
        }
        u.status = WorkStatus::Queued;
        u.claimed_by = None;
        u.result = None;
        Ok(u.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::IdentityMerge;
    use serde_json::json;

    #[test]
    fn enqueue_claim_complete() {
        let q = MemoryQueue::new();
        let node = NodeId::new();
        let u = q
            .enqueue("echo", json!({"n": 1, "api_key": "sk-secret"}))
            .unwrap();
        assert!(!u.payload.to_string().contains("sk-secret"));
        let claimed = q.claim(node).unwrap();
        assert_eq!(claimed.id, u.id);
        assert_eq!(claimed.status, WorkStatus::Claimed);
        let done = q
            .complete(u.id, node, json!({"out": 42}), &IdentityMerge)
            .unwrap();
        assert_eq!(done.status, WorkStatus::Completed);
        assert_eq!(done.result.unwrap()["out"], 42);
    }

    #[test]
    fn requeue_claimed() {
        let q = MemoryQueue::new();
        let node = NodeId::new();
        let u = q.enqueue("echo", json!({"n": 1})).unwrap();
        let _ = q.claim(node).unwrap();
        let again = q.requeue(u.id).unwrap();
        assert_eq!(again.status, WorkStatus::Queued);
        assert!(again.claimed_by.is_none());
        let claimed = q.claim(node).unwrap();
        assert_eq!(claimed.id, u.id);
    }
}
