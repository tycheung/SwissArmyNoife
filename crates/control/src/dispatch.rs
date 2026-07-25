//! Invoke dispatch: live binding check → [`Offer::invoke`].

use tracing::Instrument;
use types::{ErrorCode, InvokeReq, InvokeResp};

use crate::{invoke_span, AuditLog, BindingStore, Offer, PolicyEngine};

/// Routes an [`InvokeReq`] through a live binding to an [`Offer`].
#[derive(Debug)]
pub struct InvokeDispatcher<'a> {
    bindings: &'a BindingStore,
    policy: &'a PolicyEngine,
    audit: &'a mut AuditLog,
}

impl<'a> InvokeDispatcher<'a> {
    #[must_use]
    pub fn new(
        bindings: &'a BindingStore,
        policy: &'a PolicyEngine,
        audit: &'a mut AuditLog,
    ) -> Self {
        Self {
            bindings,
            policy,
            audit,
        }
    }

    /// Validate binding TTL / policy / offer match, then call the offer.
    pub async fn invoke(&mut self, offer: &impl Offer, mut req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        req.invoke_id = Some(invoke_id);
        let binding_id = req.binding_id;
        let catalog_offer = offer.catalog_entry().id.clone();

        let span = invoke_span(invoke_id, binding_id);
        let (offer_id, args, resp) = async {
            let record = match self.bindings.get(binding_id) {
                Ok(r) => r,
                Err(code) => {
                    let resp = InvokeResp::Error {
                        invoke_id: Some(invoke_id),
                        code,
                        message: "binding missing or expired".into(),
                    };
                    return (catalog_offer.clone(), req.args.clone(), resp);
                }
            };

            if let Err(code) = self
                .policy
                .check(record.principal.as_str(), &record.offer_id)
            {
                let resp = InvokeResp::Error {
                    invoke_id: Some(invoke_id),
                    code,
                    message: format!(
                        "principal {} denied for offer {}",
                        record.principal.as_str(),
                        record.offer_id
                    ),
                };
                return (record.offer_id.clone(), req.args.clone(), resp);
            }

            if record.offer_id != catalog_offer {
                let resp = InvokeResp::Error {
                    invoke_id: Some(invoke_id),
                    code: ErrorCode::OfferNotFound,
                    message: format!(
                        "binding offer {} does not match dispatcher offer {}",
                        record.offer_id, catalog_offer
                    ),
                };
                return (record.offer_id.clone(), req.args.clone(), resp);
            }

            if let Some(ref claimed) = req.offer {
                if claimed != &record.offer_id {
                    let resp = InvokeResp::Error {
                        invoke_id: Some(invoke_id),
                        code: ErrorCode::SchemaInvalid,
                        message: format!(
                            "req.offer {} does not match binding offer {}",
                            claimed, record.offer_id
                        ),
                    };
                    return (record.offer_id.clone(), req.args.clone(), resp);
                }
            }

            let offer_id = record.offer_id.clone();
            let args = req.args.clone();
            let resp = offer.invoke(req).await;
            (offer_id, args, resp)
        }
        .instrument(span)
        .await;

        self.audit
            .record_invoke(invoke_id, binding_id, offer_id, &args, &resp);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use serde_json::{json, Value};
    use types::{BindingId, OfferId};

    use crate::{AuditStatus, BindRequest, CatalogEntry};

    struct EchoOffer {
        entry: CatalogEntry,
    }

    impl EchoOffer {
        fn new(id: &str) -> Self {
            Self {
                entry: CatalogEntry::new(id, "0.1.0").expect("valid"),
            }
        }
    }

    impl Offer for EchoOffer {
        fn catalog_entry(&self) -> &CatalogEntry {
            &self.entry
        }

        async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
            Ok("r1".into())
        }

        async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
            Ok(())
        }

        async fn invoke(&self, req: InvokeReq) -> InvokeResp {
            let invoke_id = req.invoke_id.unwrap_or_default();
            InvokeResp::ok(invoke_id, req.args)
        }

        async fn unbind(&self, _binding_id: BindingId) -> Result<(), ErrorCode> {
            Ok(())
        }

        async fn health(&self) -> Result<(), ErrorCode> {
            Ok(())
        }
    }

    fn bind_echo(store: &mut BindingStore, offer_id: &str) -> BindingId {
        store
            .bind(BindRequest {
                offer_id: OfferId::new(offer_id).expect("valid"),
                principal: crate::Principal::local(),
                policy_json: json!({}),
                ttl: Duration::from_secs(60),
            })
            .binding_id
    }

    #[tokio::test]
    async fn dispatch_happy_path_echoes_args() {
        let mut store = BindingStore::new();
        let policy = PolicyEngine::ambient();
        let mut audit = AuditLog::new();
        let offer = EchoOffer::new("test.echo");
        let binding_id = bind_echo(&mut store, "test.echo");
        let mut dispatcher = InvokeDispatcher::new(&store, &policy, &mut audit);

        let resp = dispatcher
            .invoke(
                &offer,
                InvokeReq {
                    binding_id,
                    args: json!({"n": 7, "api_key": "sk-secret"}),
                    invoke_id: None,
                    offer: Some(OfferId::new("test.echo").expect("valid")),
                },
            )
            .await;

        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["n"], 7);
            }
            InvokeResp::Error { code, message, .. } => {
                panic!("unexpected error {code}: {message}")
            }
        }

        assert_eq!(audit.len(), 1);
        let ev = &audit.events()[0];
        assert_eq!(ev.status, AuditStatus::Ok);
        assert_eq!(ev.detail["args"]["api_key"], "[REDACTED]");
        assert_eq!(ev.detail["args"]["n"], 7);
    }

    #[tokio::test]
    async fn dispatch_rejects_missing_or_unbound() {
        let mut store = BindingStore::new();
        let policy = PolicyEngine::ambient();
        let mut audit = AuditLog::new();
        let offer = EchoOffer::new("test.echo");
        let binding_id = bind_echo(&mut store, "test.echo");
        store.unbind(binding_id).expect("unbind");

        let mut dispatcher = InvokeDispatcher::new(&store, &policy, &mut audit);
        let resp = dispatcher
            .invoke(
                &offer,
                InvokeReq {
                    binding_id,
                    args: json!({}),
                    invoke_id: None,
                    offer: None,
                },
            )
            .await;

        match resp {
            InvokeResp::Error {
                code: ErrorCode::BindingExpired,
                message,
                ..
            } => assert_eq!(message, "binding missing or expired"),
            other => panic!("expected BindingExpired, got {other:?}"),
        }
        assert_eq!(audit.len(), 1);
        assert_eq!(audit.events()[0].status, AuditStatus::Error);
    }

    #[tokio::test]
    async fn dispatch_rejects_ttl_expired_binding() {
        let mut store = BindingStore::new();
        let policy = PolicyEngine::ambient();
        let mut audit = AuditLog::new();
        let offer = EchoOffer::new("test.echo");
        let record = store.bind(BindRequest {
            offer_id: OfferId::new("test.echo").expect("valid"),
            principal: crate::Principal::local(),
            policy_json: json!({}),
            ttl: Duration::from_millis(1),
        });
        std::thread::sleep(Duration::from_millis(5));

        let mut dispatcher = InvokeDispatcher::new(&store, &policy, &mut audit);
        let resp = dispatcher
            .invoke(
                &offer,
                InvokeReq {
                    binding_id: record.binding_id,
                    args: json!({}),
                    invoke_id: None,
                    offer: None,
                },
            )
            .await;

        match resp {
            InvokeResp::Error {
                code: ErrorCode::BindingExpired,
                message,
                ..
            } => assert_eq!(message, "binding missing or expired"),
            other => panic!("expected BindingExpired, got {other:?}"),
        }
        assert_eq!(audit.events()[0].code, Some(ErrorCode::BindingExpired));
    }

    #[tokio::test]
    async fn dispatch_rejects_offer_mismatch() {
        let mut store = BindingStore::new();
        let policy = PolicyEngine::ambient();
        let mut audit = AuditLog::new();
        let wrong = EchoOffer::new("other.offer");
        let binding_id = bind_echo(&mut store, "test.echo");
        let mut dispatcher = InvokeDispatcher::new(&store, &policy, &mut audit);

        let resp = dispatcher
            .invoke(
                &wrong,
                InvokeReq {
                    binding_id,
                    args: json!({}),
                    invoke_id: None,
                    offer: None,
                },
            )
            .await;

        match resp {
            InvokeResp::Error {
                code: ErrorCode::OfferNotFound,
                ..
            } => {}
            other => panic!("expected OfferNotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_rejects_policy_denied() {
        let mut store = BindingStore::new();
        let mut policy = PolicyEngine::allowlist();
        let mut audit = AuditLog::new();
        let offer_id = OfferId::new("test.echo").expect("valid");
        policy.grant("other", &offer_id);

        let offer = EchoOffer::new("test.echo");
        let binding_id = bind_echo(&mut store, "test.echo");
        let mut dispatcher = InvokeDispatcher::new(&store, &policy, &mut audit);

        let resp = dispatcher
            .invoke(
                &offer,
                InvokeReq {
                    binding_id,
                    args: json!({}),
                    invoke_id: None,
                    offer: None,
                },
            )
            .await;

        match resp {
            InvokeResp::Error {
                code: ErrorCode::PolicyDenied,
                ..
            } => {}
            other => panic!("expected PolicyDenied, got {other:?}"),
        }
        assert_eq!(audit.events()[0].code, Some(ErrorCode::PolicyDenied));
    }
}
