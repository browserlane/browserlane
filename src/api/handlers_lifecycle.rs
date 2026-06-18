//! Phase 3 foundation: page-acquisition (browser.page / browser.pages) plus the
//! page-management routes the JS/Py client harness uses universally
//! (browser.newPage / browser.newContext / context.newPage / context.close /
//! page.activate / page.close / browser.stop) and the exported standalone page
//! functions used by the MCP agent (viewport, content, upload, mouse
//! primitives, …).

use std::sync::Arc;

use anyhow::anyhow;
use serde::Deserialize;
use serde_json::{json, Value};

use super::handlers_navigation::navigate;
use super::helpers::{call_script, check_bidi_error, resolve_element_ref, ElementParams};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::Session;

impl Router {
    /// Handles `browserlane:browser.page` — returns the first (default) browsing context.
    pub(crate) async fn handle_browser_page(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let resp = match self
            .send_internal_command(session, "browsingContext.getTree", json!({}))
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let contexts = resp
            .get("result")
            .and_then(|r| r.get("contexts"))
            .and_then(Value::as_array);
        let first = match contexts.and_then(|c| c.first()) {
            Some(c) => c,
            None => return self.send_error(session, cmd.id, &anyhow!("no browsing contexts available")),
        };

        let context = first.get("context").and_then(Value::as_str).unwrap_or("");
        let user_context = first.get("userContext").and_then(Value::as_str).unwrap_or("");
        self.send_success(
            session,
            cmd.id,
            json!({ "context": context, "userContext": user_context }),
        );
    }

    /// Handles `browserlane:browser.newPage` — creates a new tab.
    pub(crate) async fn handle_browser_new_page(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let mut params = json!({ "type": "tab" });
        let mut user_context = "default".to_string();
        // Optionally create in a specific user context.
        if let Some(uc) = cmd.params.get("userContext").and_then(Value::as_str) {
            if !uc.is_empty() {
                params["userContext"] = json!(uc);
                user_context = uc.to_string();
            }
        }

        let resp = match self.send_internal_command(session, "browsingContext.create", params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let context = match parse_context_from_create(&resp) {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        self.send_success(
            session,
            cmd.id,
            json!({ "context": context, "userContext": user_context }),
        );
    }

    /// Handles `browserlane:browser.newContext` — creates a new user context (incognito-like).
    pub(crate) async fn handle_browser_new_context(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let resp = match self
            .send_internal_command(session, "browser.createUserContext", json!({}))
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let user_context = resp
            .get("result")
            .and_then(|r| r.get("userContext"))
            .and_then(Value::as_str)
            .unwrap_or("");
        self.send_success(session, cmd.id, json!({ "userContext": user_context }));
    }

    /// Handles `browserlane:context.newPage` — creates a new tab in a user context.
    pub(crate) async fn handle_context_new_page(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let params = json!({ "type": "tab", "userContext": user_context });
        let resp = match self.send_internal_command(session, "browsingContext.create", params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let context = match parse_context_from_create(&resp) {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        self.send_success(
            session,
            cmd.id,
            json!({ "context": context, "userContext": user_context }),
        );
    }

    /// Handles `browserlane:browser.pages` — returns all browsing contexts.
    pub(crate) async fn handle_browser_pages(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let resp = match self
            .send_internal_command(session, "browsingContext.getTree", json!({}))
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let contexts = resp
            .get("result")
            .and_then(|r| r.get("contexts"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let pages: Vec<Value> = contexts
            .iter()
            .map(|ctx| {
                json!({
                    "context": ctx.get("context").and_then(Value::as_str).unwrap_or(""),
                    "url": ctx.get("url").and_then(Value::as_str).unwrap_or(""),
                    "userContext": ctx.get("userContext").and_then(Value::as_str).unwrap_or(""),
                })
            })
            .collect();

        self.send_success(session, cmd.id, json!({ "pages": pages }));
    }

    /// Handles `browserlane:context.close` — closes a user context and all its pages.
    pub(crate) async fn handle_context_close(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        if let Err(e) = self
            .send_internal_command(session, "browser.removeUserContext", json!({ "userContext": user_context }))
            .await
        {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:browser.stop` — stops the browser, then confirms to the
    /// client. Tear-down and the success reply (in that order, so the client
    /// knows Chrome is gone before it SIGTERMs the server) live in
    /// `Router::stop_session_and_reply`, next to the session/in-flight plumbing
    /// they depend on. Mirrors Go's `handleBrowserStop`.
    pub(crate) async fn handle_browser_stop(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        self.stop_session_and_reply(session, cmd.id).await;
    }

    /// Handles `browserlane:page.activate` — brings a tab to the foreground.
    pub(crate) async fn handle_page_activate(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = self
            .send_internal_command(session, "browsingContext.activate", json!({ "context": context }))
            .await
        {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.close` — closes a specific browsing context (tab).
    pub(crate) async fn handle_page_close(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = self
            .send_internal_command(session, "browsingContext.close", json!({ "context": context }))
            .await
        {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }
}

// ---------------------------------------------------------------------------
// Exported standalone lifecycle functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Information about a browsing context (page).
#[derive(Debug, Default, Clone, Deserialize)]
pub struct PageInfo {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub url: String,
    #[serde(default, rename = "userContext")]
    pub user_context: String,
}

/// Creates a new page and returns its context ID.
pub async fn new_page(s: &dyn Session, url: &str) -> anyhow::Result<String> {
    let resp = s.send_bidi_command("browsingContext.create", json!({ "type": "tab" })).await?;
    let context = parse_context_from_create(&resp)?;
    if !url.is_empty() {
        navigate(s, &context, url, "complete").await?;
    }
    Ok(context)
}

/// Returns all browsing contexts (pages).
pub async fn list_pages(s: &dyn Session) -> anyhow::Result<Vec<PageInfo>> {
    let resp = s.send_bidi_command("browsingContext.getTree", json!({})).await?;

    #[derive(Deserialize)]
    struct TreeResult {
        #[serde(default)]
        contexts: Vec<PageInfo>,
    }
    let result = resp.get("result").cloned().unwrap_or_else(|| json!({}));
    let tree: TreeResult =
        serde_json::from_value(result).map_err(|e| anyhow!("failed to parse getTree response: {e}"))?;
    Ok(tree.contexts)
}

/// Activates a browsing context (page).
pub async fn switch_page(s: &dyn Session, context_id: &str) -> anyhow::Result<()> {
    s.send_bidi_command("browsingContext.activate", json!({ "context": context_id }))
        .await
        .map(|_| ())
}

/// Closes a browsing context (page).
pub async fn close_page(s: &dyn Session, context_id: &str) -> anyhow::Result<()> {
    s.send_bidi_command("browsingContext.close", json!({ "context": context_id }))
        .await
        .map(|_| ())
}

/// Sets the viewport size of a browsing context.
pub async fn set_viewport(
    s: &dyn Session,
    context: &str,
    width: i64,
    height: i64,
    dpr: f64,
) -> anyhow::Result<()> {
    let mut params = json!({
        "context": context,
        "viewport": { "width": width, "height": height },
    });
    if dpr > 0.0 {
        params["devicePixelRatio"] = json!(dpr);
    }
    let resp = s.send_bidi_command("browsingContext.setViewport", params).await?;
    check_bidi_error(&resp)
}

/// Sets the page HTML content.
pub async fn set_content(s: &dyn Session, context: &str, html: &str) -> anyhow::Result<()> {
    let script = "(html) => {
		document.open();
		document.write(html);
		document.close();
		return 'ok';
	}";
    let args = vec![json!({ "type": "string", "value": html })];
    let resp = call_script(s, context, script, args).await?;
    check_bidi_error(&resp)
}

/// Sets files on an input[type=file] element (resolve element ref + input.setFiles).
pub async fn upload(
    s: &dyn Session,
    context: &str,
    ep: ElementParams,
    files: Vec<String>,
) -> anyhow::Result<()> {
    let shared_id = resolve_element_ref(s, context, ep).await?;
    s.send_bidi_command(
        "input.setFiles",
        json!({
            "context": context,
            "element": { "sharedId": shared_id },
            "files": files,
        }),
    )
    .await?;
    Ok(())
}

/// Moves the mouse to the given coordinates.
pub async fn mouse_move(s: &dyn Session, context: &str, x: i64, y: i64) -> anyhow::Result<()> {
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [{"type": "pointerMove", "x": x, "y": y, "duration": 0}],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Presses a mouse button.
pub async fn mouse_down(s: &dyn Session, context: &str, button: i64) -> anyhow::Result<()> {
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [{"type": "pointerDown", "button": button}],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Releases a mouse button.
pub async fn mouse_up(s: &dyn Session, context: &str, button: i64) -> anyhow::Result<()> {
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [{"type": "pointerUp", "button": button}],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Performs a click (move + down + up) at the given coordinates.
pub async fn mouse_click(s: &dyn Session, context: &str, x: i64, y: i64, button: i64) -> anyhow::Result<()> {
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [
                {"type": "pointerMove", "x": x, "y": y, "duration": 0},
                {"type": "pointerDown", "button": button},
                {"type": "pointerUp", "button": button},
            ],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Extracts the context ID from a `browsingContext.create` response.
fn parse_context_from_create(resp: &Value) -> anyhow::Result<String> {
    let context = resp
        .get("result")
        .and_then(|r| r.get("context"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if context.is_empty() {
        return Err(anyhow!("no context in create response"));
    }
    Ok(context.to_string())
}
