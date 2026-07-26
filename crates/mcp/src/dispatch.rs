//! Offer bind/unbind/invoke routing (kept out of the tool-router impl).

use crate::echo_offer::EchoOffer;
use crate::server::McpServer;
use control::{InvokeDispatcher, Offer};
use rmcp::ErrorData as McpError;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp, OfferId};

impl McpServer {
    pub(crate) async fn apply_offer_bind(
        &self,
        offer_id: &str,
        binding_id: BindingId,
        mut policy: Value,
        principal: &str,
    ) -> Result<(), McpError> {
        if let Some(obj) = policy.as_object_mut() {
            obj.entry("principal".to_string())
                .or_insert_with(|| json!(principal));
        }
        let result = match offer_id {
            "llm.chat" => self.offers.llm.bind(binding_id, policy).await,
            "llm.embed" => self.offers.llm_embed.bind(binding_id, policy).await,
            "llm.resolve" => self.offers.llm_resolve.bind(binding_id, policy).await,
            "llm.preflight" => self.offers.llm_preflight.bind(binding_id, policy).await,
            "llm.ollama.manage" => self.offers.llm_ollama_manage.bind(binding_id, policy).await,
            "llm.telemetry" => self.offers.llm_telemetry.bind(binding_id, policy).await,
            "broker.health" => self.broker_health.bind(binding_id, policy).await,
            "sandbox.exec" => self.offers.sandbox.bind(binding_id, policy).await,
            "network.egress.check" => self.offers.egress.bind(binding_id, policy).await,
            "network.egress.fetch" => self.offers.egress_fetch.bind(binding_id, policy).await,
            "memory.index" => self.offers.memory_index.bind(binding_id, policy).await,
            "memory.search" => self.offers.memory_search.bind(binding_id, policy).await,
            "memory.embed" => self.offers.memory_embed.bind(binding_id, policy).await,
            "memory.scope" => self.offers.memory_scope.bind(binding_id, policy).await,
            "research.fetch" => self.offers.research_fetch.bind(binding_id, policy).await,
            "research.brief" => self.offers.research_brief.bind(binding_id, policy).await,
            "capacity.probe" => self.offers.capacity_probe.bind(binding_id, policy).await,
            "capacity.pressure" => self.offers.capacity_pressure.bind(binding_id, policy).await,
            "capacity.fit" => self.offers.capacity_fit.bind(binding_id, policy).await,
            "compute.node" => self.offers.compute_node.bind(binding_id, policy).await,
            "compute.work" => self.offers.compute_work.bind(binding_id, policy).await,
            _ => Ok(()),
        };
        result.map_err(|code| McpError::invalid_params(format!("{code}: offer.bind failed"), None))
    }

    pub(crate) async fn apply_offer_unbind(
        &self,
        offer_id: &str,
        binding_id: BindingId,
    ) -> Result<(), ErrorCode> {
        match offer_id {
            "llm.chat" => self.offers.llm.unbind(binding_id).await,
            "llm.embed" => self.offers.llm_embed.unbind(binding_id).await,
            "llm.resolve" => self.offers.llm_resolve.unbind(binding_id).await,
            "llm.preflight" => self.offers.llm_preflight.unbind(binding_id).await,
            "llm.ollama.manage" => self.offers.llm_ollama_manage.unbind(binding_id).await,
            "llm.telemetry" => self.offers.llm_telemetry.unbind(binding_id).await,
            "broker.health" => self.broker_health.unbind(binding_id).await,
            "sandbox.exec" => self.offers.sandbox.unbind(binding_id).await,
            "network.egress.check" => self.offers.egress.unbind(binding_id).await,
            "network.egress.fetch" => self.offers.egress_fetch.unbind(binding_id).await,
            "memory.index" => self.offers.memory_index.unbind(binding_id).await,
            "memory.search" => self.offers.memory_search.unbind(binding_id).await,
            "memory.embed" => self.offers.memory_embed.unbind(binding_id).await,
            "memory.scope" => self.offers.memory_scope.unbind(binding_id).await,
            "research.fetch" => self.offers.research_fetch.unbind(binding_id).await,
            "research.brief" => self.offers.research_brief.unbind(binding_id).await,
            "capacity.probe" => self.offers.capacity_probe.unbind(binding_id).await,
            "capacity.pressure" => self.offers.capacity_pressure.unbind(binding_id).await,
            "capacity.fit" => self.offers.capacity_fit.unbind(binding_id).await,
            "compute.node" => self.offers.compute_node.unbind(binding_id).await,
            "compute.work" => self.offers.compute_work.unbind(binding_id).await,
            _ => Ok(()),
        }
    }

    pub(crate) async fn dispatch_invoke(
        &self,
        binding_id: BindingId,
        args: Value,
        offer_claim: Option<OfferId>,
    ) -> Result<InvokeResp, McpError> {
        let principal = {
            let store = self.bindings.lock().await;
            store
                .get(binding_id)
                .map_or_else(|_| "unknown".into(), |r| r.principal.as_str().to_owned())
        };
        self.rate_limiter
            .lock()
            .expect("rate limiter lock")
            .check(&principal)
            .map_err(|_| McpError::invalid_params(control::RateLimiter::deny_message(), None))?;

        let store = self.bindings.lock().await;
        let offer_id = match store.get(binding_id) {
            Ok(record) => record.offer_id.clone(),
            Err(_) => OfferId::new("unknown").expect("valid"),
        };

        let mut audit = self.audit.lock().await;
        let mut dispatcher = InvokeDispatcher::new(&store, &self.policy, &mut audit);
        let req = InvokeReq {
            binding_id,
            args,
            invoke_id: None,
            offer: offer_claim,
        };

        let resp = match offer_id.as_str() {
            "llm.chat" => dispatcher.invoke(&self.offers.llm, req).await,
            "llm.embed" => dispatcher.invoke(&self.offers.llm_embed, req).await,
            "llm.resolve" => dispatcher.invoke(&self.offers.llm_resolve, req).await,
            "llm.preflight" => dispatcher.invoke(&self.offers.llm_preflight, req).await,
            "llm.ollama.manage" => dispatcher.invoke(&self.offers.llm_ollama_manage, req).await,
            "llm.telemetry" => dispatcher.invoke(&self.offers.llm_telemetry, req).await,
            "broker.health" => dispatcher.invoke(self.broker_health.as_ref(), req).await,
            "sandbox.exec" => dispatcher.invoke(&self.offers.sandbox, req).await,
            "network.egress.check" => dispatcher.invoke(&self.offers.egress, req).await,
            "network.egress.fetch" => dispatcher.invoke(&self.offers.egress_fetch, req).await,
            "memory.index" => dispatcher.invoke(&self.offers.memory_index, req).await,
            "memory.search" => dispatcher.invoke(&self.offers.memory_search, req).await,
            "memory.embed" => dispatcher.invoke(&self.offers.memory_embed, req).await,
            "memory.scope" => dispatcher.invoke(&self.offers.memory_scope, req).await,
            "research.fetch" => dispatcher.invoke(&self.offers.research_fetch, req).await,
            "research.brief" => dispatcher.invoke(&self.offers.research_brief, req).await,
            "capacity.probe" => dispatcher.invoke(&self.offers.capacity_probe, req).await,
            "capacity.pressure" => dispatcher.invoke(&self.offers.capacity_pressure, req).await,
            "capacity.fit" => dispatcher.invoke(&self.offers.capacity_fit, req).await,
            "compute.node" => dispatcher.invoke(&self.offers.compute_node, req).await,
            "compute.work" => dispatcher.invoke(&self.offers.compute_work, req).await,
            _ => {
                let version = self
                    .catalog
                    .get(&offer_id)
                    .map_or_else(|_| "0.0.0".into(), |e| e.version.clone());
                let echo = EchoOffer::new(offer_id.as_str(), version)
                    .map_err(|code| McpError::invalid_params(format!("{code}: offer id"), None))?;
                dispatcher.invoke(&echo, req).await
            }
        };
        Ok(resp)
    }
}
