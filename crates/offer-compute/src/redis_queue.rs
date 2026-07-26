//! Redis-shaped work queue (`sak296`) — works with live Redis or in-process fake.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use types::ErrorCode;

use crate::merge::MergeHook;
use crate::node::NodeId;
use crate::queue::{WorkId, WorkQueue, WorkStatus, WorkUnit};
use crate::sanitize::sanitize_payload;

const UNITS_KEY: &str = "sak:work:units";
const ORDER_KEY: &str = "sak:work:order";

/// Minimal Redis hash/list ops used by [`RedisQueue`].
pub trait RedisBackend: Send + Sync {
    /// `HSET key field value`
    ///
    /// # Errors
    /// Backend lock or transport failures.
    fn hset(&self, key: &str, field: &str, value: &str) -> Result<(), ErrorCode>;
    /// `HGET key field`
    ///
    /// # Errors
    /// Backend lock or transport failures.
    fn hget(&self, key: &str, field: &str) -> Result<Option<String>, ErrorCode>;
    /// `HGETALL key`
    ///
    /// # Errors
    /// Backend lock or transport failures.
    fn hgetall(&self, key: &str) -> Result<Vec<(String, String)>, ErrorCode>;
    /// `RPUSH key value`
    ///
    /// # Errors
    /// Backend lock or transport failures.
    fn rpush(&self, key: &str, value: &str) -> Result<(), ErrorCode>;
    /// `LRANGE key 0 -1`
    ///
    /// # Errors
    /// Backend lock or transport failures.
    fn lrange_all(&self, key: &str) -> Result<Vec<String>, ErrorCode>;
}

/// Process-local Redis stand-in for unit tests / `COMPUTE_QUEUE=redis` without a daemon.
#[derive(Debug, Default)]
pub struct FakeRedis {
    hashes: Mutex<HashMap<String, HashMap<String, String>>>,
    lists: Mutex<HashMap<String, Vec<String>>>,
}

impl FakeRedis {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl RedisBackend for FakeRedis {
    fn hset(&self, key: &str, field: &str, value: &str) -> Result<(), ErrorCode> {
        let mut hashes = self.hashes.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        hashes
            .entry(key.to_owned())
            .or_default()
            .insert(field.to_owned(), value.to_owned());
        Ok(())
    }

    fn hget(&self, key: &str, field: &str) -> Result<Option<String>, ErrorCode> {
        let hashes = self.hashes.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(hashes.get(key).and_then(|h| h.get(field).cloned()))
    }

    fn hgetall(&self, key: &str) -> Result<Vec<(String, String)>, ErrorCode> {
        let hashes = self.hashes.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(hashes
            .get(key)
            .map(|h| h.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default())
    }

    fn rpush(&self, key: &str, value: &str) -> Result<(), ErrorCode> {
        let mut lists = self.lists.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        lists
            .entry(key.to_owned())
            .or_default()
            .push(value.to_owned());
        Ok(())
    }

    fn lrange_all(&self, key: &str) -> Result<Vec<String>, ErrorCode> {
        let lists = self.lists.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        Ok(lists.get(key).cloned().unwrap_or_default())
    }
}

/// Work queue persisted in Redis hashes/lists (or [`FakeRedis`]).
pub struct RedisQueue {
    backend: Box<dyn RedisBackend>,
    /// Serializes claim/complete (`FakeRedis` is not multi-key atomic).
    lock: Mutex<()>,
}

impl RedisQueue {
    /// Wrap any backend (fake or live).
    #[must_use]
    pub fn new(backend: Box<dyn RedisBackend>) -> Self {
        Self {
            backend,
            lock: Mutex::new(()),
        }
    }

    /// In-process fake Redis (default when `COMPUTE_QUEUE=redis` and no `REDIS_URL`).
    #[must_use]
    pub fn fake() -> Self {
        Self::new(Box::new(FakeRedis::new()))
    }

    /// Connect to `REDIS_URL` when the `redis` feature is enabled.
    ///
    /// # Errors
    /// Missing URL, connection failure, or feature disabled.
    pub fn from_env() -> Result<Self, ErrorCode> {
        let url = std::env::var("REDIS_URL").map_err(|_| ErrorCode::SchemaInvalid)?;
        if url.is_empty() {
            return Err(ErrorCode::SchemaInvalid);
        }
        #[cfg(feature = "redis")]
        {
            return Ok(Self::new(Box::new(LiveRedis::connect(&url)?)));
        }
        #[cfg(not(feature = "redis"))]
        {
            let _ = url;
            Err(ErrorCode::SchemaInvalid)
        }
    }
}

impl WorkQueue for RedisQueue {
    fn enqueue(&self, kind: &str, payload: Value) -> Result<WorkUnit, ErrorCode> {
        let _g = self.lock.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let unit = WorkUnit {
            id: WorkId::new(),
            kind: kind.to_owned(),
            payload: sanitize_payload(payload),
            status: WorkStatus::Queued,
            claimed_by: None,
            result: None,
        };
        let json = serde_json::to_string(&unit).map_err(|_| ErrorCode::SchemaInvalid)?;
        self.backend.hset(UNITS_KEY, &unit.id.to_string(), &json)?;
        self.backend.rpush(ORDER_KEY, &unit.id.to_string())?;
        Ok(unit)
    }

    fn claim(&self, node: NodeId) -> Result<WorkUnit, ErrorCode> {
        let _g = self.lock.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let order = self.backend.lrange_all(ORDER_KEY)?;
        for id in order {
            let Some(raw) = self.backend.hget(UNITS_KEY, &id)? else {
                continue;
            };
            let mut unit: WorkUnit =
                serde_json::from_str(&raw).map_err(|_| ErrorCode::SchemaInvalid)?;
            if unit.status != WorkStatus::Queued {
                continue;
            }
            unit.status = WorkStatus::Claimed;
            unit.claimed_by = Some(node);
            let json = serde_json::to_string(&unit).map_err(|_| ErrorCode::SchemaInvalid)?;
            self.backend.hset(UNITS_KEY, &id, &json)?;
            return Ok(unit);
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
        let _g = self.lock.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let raw = self
            .backend
            .hget(UNITS_KEY, &work_id.to_string())?
            .ok_or(ErrorCode::OfferNotFound)?;
        let mut unit: WorkUnit =
            serde_json::from_str(&raw).map_err(|_| ErrorCode::SchemaInvalid)?;
        if unit.status != WorkStatus::Claimed || unit.claimed_by != Some(node) {
            return Err(ErrorCode::PolicyDenied);
        }
        let merged = merge.merge(&unit.payload, &result)?;
        unit.result = Some(sanitize_payload(merged));
        unit.status = WorkStatus::Completed;
        let json = serde_json::to_string(&unit).map_err(|_| ErrorCode::SchemaInvalid)?;
        self.backend.hset(UNITS_KEY, &work_id.to_string(), &json)?;
        Ok(unit)
    }

    fn get(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode> {
        let _g = self.lock.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let raw = self
            .backend
            .hget(UNITS_KEY, &work_id.to_string())?
            .ok_or(ErrorCode::OfferNotFound)?;
        serde_json::from_str(&raw).map_err(|_| ErrorCode::SchemaInvalid)
    }

    fn list(&self, limit: usize) -> Result<Vec<WorkUnit>, ErrorCode> {
        let _g = self.lock.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let order = self.backend.lrange_all(ORDER_KEY)?;
        let mut out = Vec::new();
        for id in order.into_iter().rev().take(limit) {
            if let Some(raw) = self.backend.hget(UNITS_KEY, &id)? {
                let unit: WorkUnit =
                    serde_json::from_str(&raw).map_err(|_| ErrorCode::SchemaInvalid)?;
                out.push(unit);
            }
        }
        Ok(out)
    }

    fn requeue(&self, work_id: WorkId) -> Result<WorkUnit, ErrorCode> {
        let _g = self.lock.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        let raw = self
            .backend
            .hget(UNITS_KEY, &work_id.to_string())?
            .ok_or(ErrorCode::OfferNotFound)?;
        let mut unit: WorkUnit =
            serde_json::from_str(&raw).map_err(|_| ErrorCode::SchemaInvalid)?;
        if unit.status != WorkStatus::Claimed && unit.status != WorkStatus::Failed {
            return Err(ErrorCode::PolicyDenied);
        }
        unit.status = WorkStatus::Queued;
        unit.claimed_by = None;
        unit.result = None;
        let json = serde_json::to_string(&unit).map_err(|_| ErrorCode::SchemaInvalid)?;
        self.backend.hset(UNITS_KEY, &work_id.to_string(), &json)?;
        Ok(unit)
    }
}

#[cfg(feature = "redis")]
struct LiveRedis {
    conn: Mutex<redis::Connection>,
}

#[cfg(feature = "redis")]
impl LiveRedis {
    fn connect(url: &str) -> Result<Self, ErrorCode> {
        let client = redis::Client::open(url).map_err(|_| ErrorCode::SchemaInvalid)?;
        let conn = client
            .get_connection()
            .map_err(|_| ErrorCode::ProviderUnreachable)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

#[cfg(feature = "redis")]
impl RedisBackend for LiveRedis {
    fn hset(&self, key: &str, field: &str, value: &str) -> Result<(), ErrorCode> {
        use redis::Commands;
        let mut conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.hset::<_, _, _, ()>(key, field, value)
            .map_err(|_| ErrorCode::ProviderUnreachable)
    }

    fn hget(&self, key: &str, field: &str) -> Result<Option<String>, ErrorCode> {
        use redis::Commands;
        let mut conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.hget(key, field)
            .map_err(|_| ErrorCode::ProviderUnreachable)
    }

    fn hgetall(&self, key: &str) -> Result<Vec<(String, String)>, ErrorCode> {
        use redis::Commands;
        let mut conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.hgetall(key)
            .map_err(|_| ErrorCode::ProviderUnreachable)
    }

    fn rpush(&self, key: &str, value: &str) -> Result<(), ErrorCode> {
        use redis::Commands;
        let mut conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.rpush::<_, _, ()>(key, value)
            .map_err(|_| ErrorCode::ProviderUnreachable)
    }

    fn lrange_all(&self, key: &str) -> Result<Vec<String>, ErrorCode> {
        use redis::Commands;
        let mut conn = self.conn.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        conn.lrange(key, 0, -1)
            .map_err(|_| ErrorCode::ProviderUnreachable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::IdentityMerge;
    use serde_json::json;

    #[test]
    fn fake_redis_enqueue_claim_complete() {
        let q = RedisQueue::fake();
        let node = NodeId::new();
        let u = q
            .enqueue("echo", json!({"n": 1, "api_key": "sk-secret"}))
            .unwrap();
        assert!(!u.payload.to_string().contains("sk-secret"));
        let claimed = q.claim(node).unwrap();
        assert_eq!(claimed.id, u.id);
        let done = q
            .complete(u.id, node, json!({"out": 7}), &IdentityMerge)
            .unwrap();
        assert_eq!(done.status, WorkStatus::Completed);
        assert_eq!(done.result.unwrap()["out"], 7);
        assert_eq!(q.list(5).unwrap().len(), 1);
    }
}
