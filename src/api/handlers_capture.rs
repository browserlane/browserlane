//! Phase 3 capture cluster: page screenshot / pdf routes and the exported
//! standalone capture functions used by the MCP agent. The element-screenshot
//! route lives in handlers_state.go (and is ported there).

use std::sync::Arc;

use anyhow::anyhow;
use serde_json::{json, Value};

use super::helpers::check_bidi_error;
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::Session;

impl Router {
    /// Handles `browserlane:page.screenshot` — captures a page screenshot (base64 PNG).
    /// Options: fullPage (boolean), clip ({x, y, width, height}).
    pub(crate) async fn handle_page_screenshot(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let mut ss_params = json!({ "context": context });

        // fullPage: set origin to "document".
        if cmd.params.get("fullPage").and_then(Value::as_bool) == Some(true) {
            ss_params["origin"] = json!("document");
        }

        // clip: {x, y, width, height}.
        if let Some(clip) = cmd.params.get("clip").and_then(Value::as_object) {
            ss_params["clip"] = json!({
                "type": "box",
                "x": clip.get("x").cloned().unwrap_or(Value::Null),
                "y": clip.get("y").cloned().unwrap_or(Value::Null),
                "width": clip.get("width").cloned().unwrap_or(Value::Null),
                "height": clip.get("height").cloned().unwrap_or(Value::Null),
            });
        }

        let resp = match self.send_internal_command(session, "browsingContext.captureScreenshot", ss_params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }

        let data = resp.get("result").and_then(|r| r.get("data")).and_then(Value::as_str).unwrap_or("");
        self.send_success(session, cmd.id, json!({ "data": data }));
    }

    /// Handles `browserlane:page.pdf` — prints the page to PDF (base64).
    pub(crate) async fn handle_page_pdf(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let resp = match self
            .send_internal_command(session, "browsingContext.print", json!({ "context": context }))
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }

        let data = resp.get("result").and_then(|r| r.get("data")).and_then(Value::as_str).unwrap_or("");
        self.send_success(session, cmd.id, json!({ "data": data }));
    }
}

// ---------------------------------------------------------------------------
// Exported standalone capture functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Captures a page screenshot and returns base64-encoded PNG data.
pub async fn screenshot(s: &dyn Session, context: &str, full_page: bool) -> anyhow::Result<String> {
    let mut ss_params = json!({ "context": context });
    if full_page {
        ss_params["origin"] = json!("document");
    }

    let resp = s.send_bidi_command("browsingContext.captureScreenshot", ss_params).await?;
    check_bidi_error(&resp)?;

    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("screenshot parse failed: no data"))?;
    Ok(data.to_string())
}

/// Prints the page to PDF and returns base64-encoded PDF data.
pub async fn print_to_pdf(s: &dyn Session, context: &str) -> anyhow::Result<String> {
    let resp = s.send_bidi_command("browsingContext.print", json!({ "context": context })).await?;
    check_bidi_error(&resp)?;

    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("pdf parse failed: no data"))?;
    Ok(data.to_string())
}
