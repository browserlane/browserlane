use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::helpers::BoxInfo;
use super::router::{BrowserSession, Router};
use crate::bidi::Client;

/// Abstracts BiDi communication so that both the proxy (WebSocket/pipe router)
/// and the MCP agent (direct bidi.Client) share the same automation logic.
#[async_trait]
pub trait Session: Send + Sync {
    /// Sends a BiDi command and returns the full response JSON.
    async fn send_bidi_command(&self, method: &str, params: Value) -> anyhow::Result<Value>;

    /// Like `send_bidi_command` but with a custom timeout.
    async fn send_bidi_command_with_timeout(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> anyhow::Result<Value>;

    /// Returns a browsing context ID (current if tracked, else the first one).
    async fn get_context_id(&self) -> anyhow::Result<String>;

    /// Stores the bounding box of the last resolved element (for recording).
    fn set_last_element_box(&self, box_: BoxInfo);
}

// ---------------------------------------------------------------------------
// ApiSession — adapts Router + BrowserSession to Session.
// ---------------------------------------------------------------------------

/// Wraps a Router and BrowserSession pair so shared functions can send internal
/// commands through the Session interface.
pub struct ApiSession {
    pub router: Arc<Router>,
    pub session: Arc<BrowserSession>,
    /// Optional explicit context override.
    pub context: String,
}

/// Creates an ApiSession.
pub fn new_api_session(router: Arc<Router>, session: Arc<BrowserSession>, context: &str) -> ApiSession {
    ApiSession {
        router,
        session,
        context: context.to_string(),
    }
}

#[async_trait]
impl Session for ApiSession {
    async fn send_bidi_command(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.router
            .send_internal_command(&self.session, method, params)
            .await
    }

    async fn send_bidi_command_with_timeout(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> anyhow::Result<Value> {
        self.router
            .send_internal_command_with_timeout(&self.session, method, params, timeout)
            .await
    }

    async fn get_context_id(&self) -> anyhow::Result<String> {
        if !self.context.is_empty() {
            return Ok(self.context.clone());
        }
        self.router.get_context(&self.session).await
    }

    fn set_last_element_box(&self, box_: BoxInfo) {
        self.session.set_last_element_box(box_);
    }
}

// ---------------------------------------------------------------------------
// AgentSession — adapts bidi::Client to Session.
// ---------------------------------------------------------------------------

/// Wraps a bidi.Client so shared functions can send BiDi commands through the
/// Session interface. The Client already surfaces BiDi error responses as Rust
/// errors, so check_bidi_error on wrapped responses is a safe no-op.
pub struct AgentSession {
    pub client: Arc<Client>,
    /// Optional explicit context override (active tab).
    pub context: String,
    /// Optional callback invoked when an element box is set.
    pub on_box_set: Option<Box<dyn Fn(BoxInfo) + Send + Sync>>,
}

/// Creates an AgentSession.
pub fn new_agent_session(client: Arc<Client>) -> AgentSession {
    AgentSession {
        client,
        context: String::new(),
        on_box_set: None,
    }
}

#[async_trait]
impl Session for AgentSession {
    async fn send_bidi_command(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let msg = self.client.send_command(method, params).await?;
        // Wrap as {"result": <msg.result>} to match the proxy response format
        // that parse_script_result / check_bidi_error expect.
        Ok(json!({ "result": msg.result }))
    }

    async fn send_bidi_command_with_timeout(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> anyhow::Result<Value> {
        let msg = self
            .client
            .send_command_with_timeout(method, params, timeout)
            .await?;
        Ok(json!({ "result": msg.result }))
    }

    async fn get_context_id(&self) -> anyhow::Result<String> {
        if !self.context.is_empty() {
            return Ok(self.context.clone());
        }
        let tree = self
            .client
            .get_tree()
            .await
            .map_err(|e| anyhow!("failed to get browsing context: {e}"))?;
        if tree.contexts.is_empty() {
            return Err(anyhow!("no browsing contexts available"));
        }
        Ok(tree.contexts[0].context.clone())
    }

    fn set_last_element_box(&self, box_: BoxInfo) {
        if let Some(cb) = &self.on_box_set {
            cb(box_);
        }
    }
}
