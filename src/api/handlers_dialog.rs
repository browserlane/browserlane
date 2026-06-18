//! Phase 3 dialog cluster: the `browserlane:dialog.accept` / `browserlane:dialog.dismiss`
//! routes plus the exported standalone dialog functions used by the MCP agent.
//! Dialog *events* (`browsingContext.userPromptOpened`) are already subscribed
//! and forwarded by the router's generic event loop; auto-dismiss-when-no-handler
//! is governed by the session capabilities + the client library, exactly as in Go.

use std::sync::Arc;

use serde_json::{json, Value};

use super::helpers::check_bidi_error;
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::Session;

impl Router {
    /// Handles `browserlane:dialog.accept` — accepts a user prompt (alert/confirm/prompt).
    pub(crate) async fn handle_dialog_accept(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        let mut params = json!({
            "context": context,
            "accept": true,
        });
        if let Some(user_text) = cmd.params.get("userText").and_then(Value::as_str) {
            params["userText"] = json!(user_text);
        }

        let resp = match self
            .send_internal_command(session, "browsingContext.handleUserPrompt", params)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        if let Err(e) = check_bidi_error(&resp) {
            self.send_error(session, cmd.id, &e);
            return;
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:dialog.dismiss` — dismisses a user prompt.
    pub(crate) async fn handle_dialog_dismiss(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        let params = json!({
            "context": context,
            "accept": false,
        });

        let resp = match self
            .send_internal_command(session, "browsingContext.handleUserPrompt", params)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        if let Err(e) = check_bidi_error(&resp) {
            self.send_error(session, cmd.id, &e);
            return;
        }

        self.send_success(session, cmd.id, json!({}));
    }
}

// ---------------------------------------------------------------------------
// Exported standalone dialog functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Accepts a user prompt (alert/confirm/prompt).
pub async fn dialog_accept(s: &dyn Session, context: &str, user_text: &str) -> anyhow::Result<()> {
    let mut params = json!({
        "context": context,
        "accept": true,
    });
    if !user_text.is_empty() {
        params["userText"] = json!(user_text);
    }

    let resp = s
        .send_bidi_command("browsingContext.handleUserPrompt", params)
        .await?;
    check_bidi_error(&resp)
}

/// Dismisses a user prompt.
pub async fn dialog_dismiss(s: &dyn Session, context: &str) -> anyhow::Result<()> {
    let params = json!({
        "context": context,
        "accept": false,
    });

    let resp = s
        .send_bidi_command("browsingContext.handleUserPrompt", params)
        .await?;
    check_bidi_error(&resp)
}
