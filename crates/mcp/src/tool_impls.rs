//! Tool handler bodies extracted from `server.rs` (`refactor:mcp-server-split`).

use crate::server::McpServer;
use crate::tool_args::{
    CapacityFitArgs, CapacityPressureArgs, CapacityProbeArgs, ComputeNodeArgs, ComputeWorkArgs,
    EgressCheckArgs, EgressFetchArgs, FsEditArgs, FsGrepArgs, FsReadArgs, FsWriteArgs,
    MemoryEmbedArgs, MemoryIndexArgs, MemorySearchArgs, ResearchBriefArgs, ResearchFetchArgs,
    ShellExecArgs,
};
use crate::util::{parse_binding_id, serialize_resp};
use crate::workspace_tools::{fs_err, mode_label, parse_read_mode, shell_err};
use rmcp::ErrorData as McpError;
use serde_json::json;
use types::OfferId;

impl McpServer {
    pub(crate) fn fs_read_inner(&self, args: FsReadArgs) -> Result<String, McpError> {
        let FsReadArgs { path, mode } = args;
        let mode = parse_read_mode(mode.as_deref())?;
        let text = self.fs.read_mode(&path, mode).map_err(|e| fs_err(&e))?;
        Ok(json!({
            "path": path,
            "mode": mode_label(mode),
            "text": text
        })
        .to_string())
    }

    pub(crate) fn fs_write_inner(&self, args: FsWriteArgs) -> Result<String, McpError> {
        let FsWriteArgs { path, content } = args;
        self.fs.write(&path, &content).map_err(|e| fs_err(&e))?;
        Ok(json!({ "path": path, "written": true }).to_string())
    }

    pub(crate) fn fs_edit_inner(&self, args: FsEditArgs) -> Result<String, McpError> {
        let FsEditArgs { path, old, new } = args;
        self.fs.edit(&path, &old, &new).map_err(|e| fs_err(&e))?;
        Ok(json!({ "path": path, "edited": true }).to_string())
    }

    pub(crate) fn fs_grep_inner(&self, args: FsGrepArgs) -> Result<String, McpError> {
        let FsGrepArgs { path, pattern } = args;
        let hits = self.fs.grep(&path, &pattern).map_err(|e| fs_err(&e))?;
        let hits: Vec<_> = hits
            .into_iter()
            .map(|h| json!({ "line": h.line, "text": h.text }))
            .collect();
        Ok(json!({ "path": path, "hits": hits }).to_string())
    }

    pub(crate) fn shell_exec_inner(&self, args: ShellExecArgs) -> Result<String, McpError> {
        let ShellExecArgs { argv, cwd } = args;
        let out = self.shell.exec(argv, cwd).map_err(|e| shell_err(&e))?;
        Ok(json!({
            "exit_code": out.exit_code,
            "stdout": out.stdout,
            "stderr": out.stderr
        })
        .to_string())
    }

    pub(crate) async fn egress_check_inner(
        &self,
        args: EgressCheckArgs,
    ) -> Result<String, McpError> {
        let EgressCheckArgs { binding_id, url } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let claim = OfferId::new("network.egress.check").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, json!({ "url": url }), Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn egress_fetch_inner(
        &self,
        args: EgressFetchArgs,
    ) -> Result<String, McpError> {
        let EgressFetchArgs { binding_id, url } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let claim = OfferId::new("network.egress.fetch").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, json!({ "url": url }), Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn memory_embed_inner(
        &self,
        args: MemoryEmbedArgs,
    ) -> Result<String, McpError> {
        let MemoryEmbedArgs {
            binding_id,
            inputs,
            model,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let invoke_args = json!({ "inputs": inputs, "model": model });
        let claim = OfferId::new("memory.embed").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, invoke_args, Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn memory_index_inner(
        &self,
        args: MemoryIndexArgs,
    ) -> Result<String, McpError> {
        let MemoryIndexArgs {
            binding_id,
            documents,
            scope_key,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let docs: Vec<_> = documents
            .into_iter()
            .map(|d| json!({ "id": d.id, "text": d.text }))
            .collect();
        let invoke_args = json!({ "documents": docs, "scope_key": scope_key });
        let claim = OfferId::new("memory.index").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, invoke_args, Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn memory_search_inner(
        &self,
        args: MemorySearchArgs,
    ) -> Result<String, McpError> {
        let MemorySearchArgs {
            binding_id,
            query,
            k,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let invoke_args = json!({ "query": query, "k": k.unwrap_or(5) });
        let claim = OfferId::new("memory.search").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, invoke_args, Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn research_fetch_inner(
        &self,
        args: ResearchFetchArgs,
    ) -> Result<String, McpError> {
        let ResearchFetchArgs { binding_id, url } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let claim = OfferId::new("research.fetch").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, json!({ "url": url }), Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn research_brief_inner(
        &self,
        args: ResearchBriefArgs,
    ) -> Result<String, McpError> {
        let ResearchBriefArgs {
            binding_id,
            action,
            id,
            title,
            body,
            source_url,
            limit,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let invoke_args = json!({
            "action": action,
            "id": id,
            "title": title,
            "body": body,
            "source_url": source_url,
            "limit": limit.unwrap_or(20),
        });
        let claim = OfferId::new("research.brief").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, invoke_args, Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn capacity_probe_inner(
        &self,
        args: CapacityProbeArgs,
    ) -> Result<String, McpError> {
        let CapacityProbeArgs { binding_id } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let claim = OfferId::new("capacity.probe").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, json!({}), Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn capacity_pressure_inner(
        &self,
        args: CapacityPressureArgs,
    ) -> Result<String, McpError> {
        let CapacityPressureArgs { binding_id } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let claim = OfferId::new("capacity.pressure").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, json!({}), Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn capacity_fit_inner(
        &self,
        args: CapacityFitArgs,
    ) -> Result<String, McpError> {
        let CapacityFitArgs {
            binding_id,
            candidates,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let cands: Vec<_> = candidates
            .into_iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "ram_mb": c.ram_mb,
                    "vram_mb": c.vram_mb.unwrap_or(0),
                })
            })
            .collect();
        let invoke_args = json!({ "candidates": cands });
        let claim = OfferId::new("capacity.fit").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, invoke_args, Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn compute_node_inner(
        &self,
        args: ComputeNodeArgs,
    ) -> Result<String, McpError> {
        let ComputeNodeArgs {
            binding_id,
            action,
            label,
            caps,
            node_id,
            stale_secs,
            session_id,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let invoke_args = json!({
            "action": action,
            "label": label,
            "caps": caps,
            "node_id": node_id,
            "stale_secs": stale_secs,
            "session_id": session_id,
        });
        let claim = OfferId::new("compute.node").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, invoke_args, Some(claim))
            .await?;
        serialize_resp(&resp)
    }

    pub(crate) async fn compute_work_inner(
        &self,
        args: ComputeWorkArgs,
    ) -> Result<String, McpError> {
        let ComputeWorkArgs {
            binding_id,
            action,
            kind,
            payload,
            node_id,
            work_id,
            result,
            run_id,
            stage_name,
            status,
            limit,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let invoke_args = json!({
            "action": action,
            "kind": kind,
            "payload": payload,
            "node_id": node_id,
            "work_id": work_id,
            "result": result,
            "run_id": run_id,
            "stage_name": stage_name,
            "status": status,
            "limit": limit,
        });
        let claim = OfferId::new("compute.work").expect("valid");
        let resp = self
            .dispatch_invoke(binding_id, invoke_args, Some(claim))
            .await?;
        serialize_resp(&resp)
    }
}
