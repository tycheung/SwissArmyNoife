//! Control-plane traits for `SwissArmyNoife`.

mod api_key;
mod audit;
mod binding;
mod budget;
mod catalog;
mod dispatch;
mod health;
mod idempotency;
mod meter;
mod offer;
mod policy;
mod policy_templates;
mod principal;
mod provision;
mod rate_limit;
mod risk;
mod trace;

pub use api_key::{hash_api_key_secret, ApiKeyInfo, ApiKeyRow, ApiKeyStore};
pub use audit::{redact_json, AuditEvent, AuditLog, AuditStatus};
pub use binding::{BindRequest, BindingRecord, BindingStore};
pub use budget::{BudgetLedger, BudgetLimits, BudgetUsage};
pub use catalog::CatalogRegistry;
pub use dispatch::InvokeDispatcher;
pub use health::{BrokerHealthOffer, EmptyHealthSnapshot, HealthSnapshot};
pub use idempotency::IdempotencyStore;
pub use meter::MeterSnapshot;
pub use offer::{CatalogEntry, Offer};
pub use policy::PolicyEngine;
pub use policy_templates::{list_template_names, resolve_policy};
pub use principal::{Principal, PrincipalKind};
pub use provision::{ProvisionStore, ResourceRecord, ResourceState};
pub use rate_limit::{RateLimitStatus, RateLimiter};
pub use risk::{RiskCaps, RiskLedger, RiskUsage};
pub use trace::invoke_span;
