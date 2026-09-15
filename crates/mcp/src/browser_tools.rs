//! Typed MCP tools for `browser.session` (separate router to keep `server.rs` under `FAIL_LOC`).

use rmcp::{
    handler::server::wrapper::Parameters, service::RequestContext, tool, tool_router,
    ErrorData as McpError, RoleServer,
};
use serde_json::json;

use crate::progress::notify_progress;
use crate::server::McpServer;
use crate::tool_args::{
    BrowserCdpArgs, BrowserDragArgs, BrowserFailureReportArgs, BrowserLockArgs, BrowserLogTailArgs,
    BrowserMouseXyArgs, BrowserNavigateArgs, BrowserPressKeyArgs, BrowserRefArgs,
    BrowserScreenshotArgs, BrowserScrollArgs, BrowserSelectArgs, BrowserSnapshotArgs,
    BrowserTabsArgs, BrowserTypeArgs,
};

#[tool_router(router = browser_tool_router, vis = "pub(crate)")]
impl McpServer {
    /// Typed invoke for `browser.session` navigate.
    #[tool(
        description = "Navigate browser.session to a URL under egress policy (returns InvokeResp)"
    )]
    async fn browser_navigate(
        &self,
        Parameters(args): Parameters<BrowserNavigateArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<String, McpError> {
        notify_progress(&context, 0.0, Some(1.0), "browser_navigate start").await;
        let out = self.browser_navigate_inner(args).await?;
        notify_progress(&context, 1.0, Some(1.0), "browser_navigate done").await;
        Ok(out)
    }

    /// Typed invoke for `browser.session` snapshot.
    #[tool(
        description = "Capture a11y text snapshot with refs for the current browser.session page"
    )]
    async fn browser_snapshot(
        &self,
        Parameters(args): Parameters<BrowserSnapshotArgs>,
    ) -> Result<String, McpError> {
        self.browser_snapshot_inner(args).await
    }

    #[tool(description = "Click a snapshot ref in browser.session")]
    async fn browser_click(
        &self,
        Parameters(args): Parameters<BrowserRefArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "click", "ref": args.r#ref }),
        )
        .await
    }

    #[tool(description = "Type into a snapshot ref in browser.session")]
    async fn browser_type(
        &self,
        Parameters(args): Parameters<BrowserTypeArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "type", "ref": args.r#ref, "text": args.text }),
        )
        .await
    }

    #[tool(description = "Fill a snapshot ref in browser.session")]
    async fn browser_fill(
        &self,
        Parameters(args): Parameters<BrowserTypeArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "fill", "ref": args.r#ref, "text": args.text }),
        )
        .await
    }

    #[tool(description = "Press a key in browser.session (optional ref focus)")]
    async fn browser_press_key(
        &self,
        Parameters(args): Parameters<BrowserPressKeyArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "press_key", "key": args.key, "ref": args.r#ref }),
        )
        .await
    }

    #[tool(description = "Scroll in browser.session (ref or wheel deltas)")]
    async fn browser_scroll(
        &self,
        Parameters(args): Parameters<BrowserScrollArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({
                "action": "scroll",
                "ref": args.r#ref,
                "delta_x": args.delta_x,
                "delta_y": args.delta_y
            }),
        )
        .await
    }

    #[tool(description = "Select option(s) on a snapshot ref in browser.session")]
    async fn browser_select_option(
        &self,
        Parameters(args): Parameters<BrowserSelectArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({
                "action": "select_option",
                "ref": args.r#ref,
                "value": args.value,
                "values": args.values
            }),
        )
        .await
    }

    #[tool(description = "Drag one snapshot ref onto another in browser.session")]
    async fn browser_drag(
        &self,
        Parameters(args): Parameters<BrowserDragArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({
                "action": "drag",
                "ref": args.r#ref,
                "target_ref": args.target_ref
            }),
        )
        .await
    }

    #[tool(description = "Click viewport coordinates in browser.session")]
    async fn browser_mouse_click_xy(
        &self,
        Parameters(args): Parameters<BrowserMouseXyArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "mouse_click_xy", "x": args.x, "y": args.y }),
        )
        .await
    }

    #[tool(description = "Screenshot browser.session page to CONFIG_DIR artifact path")]
    async fn browser_take_screenshot(
        &self,
        Parameters(args): Parameters<BrowserScreenshotArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "take_screenshot", "full_page": args.full_page }),
        )
        .await
    }

    #[tool(description = "Highlight a snapshot ref in browser.session")]
    async fn browser_highlight(
        &self,
        Parameters(args): Parameters<BrowserRefArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "highlight", "ref": args.r#ref }),
        )
        .await
    }

    #[tool(description = "Get bounding box for a snapshot ref in browser.session")]
    async fn browser_get_bounding_box(
        &self,
        Parameters(args): Parameters<BrowserRefArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "get_bounding_box", "ref": args.r#ref }),
        )
        .await
    }

    #[tool(description = "List/create/select/close tabs in browser.session")]
    async fn browser_tabs(
        &self,
        Parameters(args): Parameters<BrowserTabsArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({
                "action": "tabs",
                "tabs_action": args.action,
                "index": args.index,
                "url": args.url
            }),
        )
        .await
    }

    #[tool(description = "Lock or unlock browser.session interactions")]
    async fn browser_lock(
        &self,
        Parameters(args): Parameters<BrowserLockArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "lock", "lock_action": args.action }),
        )
        .await
    }

    #[tool(description = "Tail buffered console/pageerror events for browser.session")]
    async fn browser_console(
        &self,
        Parameters(args): Parameters<BrowserLogTailArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "console", "limit": args.limit }),
        )
        .await
    }

    #[tool(description = "Tail buffered failed/network events for browser.session")]
    async fn browser_network(
        &self,
        Parameters(args): Parameters<BrowserLogTailArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "network", "limit": args.limit }),
        )
        .await
    }

    #[tool(
        description = "Package screenshot + console/network snippet for a browser.session failure"
    )]
    async fn browser_failure_report(
        &self,
        Parameters(args): Parameters<BrowserFailureReportArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({ "action": "failure_report", "step": args.step }),
        )
        .await
    }

    #[tool(description = "Send a CDP method (Input.* denied) on browser.session")]
    async fn browser_cdp(
        &self,
        Parameters(args): Parameters<BrowserCdpArgs>,
    ) -> Result<String, McpError> {
        self.browser_action(
            &args.binding_id,
            json!({
                "action": "cdp",
                "method": args.method,
                "params": args.params.unwrap_or(json!({}))
            }),
        )
        .await
    }
}
