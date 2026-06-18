//! Phase 3 frames cluster: browserlane:page.frames (list child frames) and
//! browserlane:page.frame (find a frame by name or URL substring). Chrome's BiDi
//! getTree doesn't return the iframe name, so we resolve window.name per frame.

use std::sync::Arc;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::helpers::{check_bidi_error, eval_simple_script};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::{new_api_session, Session};

/// A browsing context from getTree.
#[derive(Debug, Deserialize)]
struct ContextInfo {
    #[serde(default)]
    context: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    children: Option<Vec<ContextInfo>>,
}

#[derive(Debug, Default, Deserialize)]
struct GetTreeInner {
    #[serde(default)]
    contexts: Vec<ContextInfo>,
}

#[derive(Debug, Default, Deserialize)]
struct GetTreeResult {
    #[serde(default)]
    result: GetTreeInner,
}

/// Recursively collects all child frames into a flat list of `{context, url}`.
fn collect_frames(contexts: &[ContextInfo]) -> Vec<Map<String, Value>> {
    let mut frames = Vec::new();
    for ctx in contexts {
        let mut m = Map::new();
        m.insert("context".to_string(), json!(ctx.context));
        m.insert("url".to_string(), json!(ctx.url));
        frames.push(m);
        if let Some(children) = &ctx.children {
            if !children.is_empty() {
                frames.extend(collect_frames(children));
            }
        }
    }
    frames
}

impl Router {
    /// Gets the frame tree for a context and returns flattened child frames.
    async fn get_frame_tree(
        &self,
        session: &Arc<BrowserSession>,
        context: &str,
    ) -> anyhow::Result<Vec<Map<String, Value>>> {
        let resp = self
            .send_internal_command(session, "browsingContext.getTree", json!({ "root": context }))
            .await?;
        check_bidi_error(&resp)?;

        let parsed: GetTreeResult = serde_json::from_value(resp)
            .map_err(|e| anyhow!("failed to parse getTree response: {e}"))?;

        let frames = match parsed.result.contexts.first() {
            Some(root) => collect_frames(root.children.as_deref().unwrap_or(&[])),
            None => Vec::new(),
        };
        Ok(frames)
    }

    /// Evaluates window.name in each frame context to populate names.
    async fn resolve_frame_names(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        frames: &mut [Map<String, Value>],
    ) {
        for f in frames.iter_mut() {
            let ctx = f.get("context").and_then(Value::as_str).unwrap_or("").to_string();
            if ctx.is_empty() {
                continue;
            }
            let s = new_api_session(Arc::clone(self), Arc::clone(session), &ctx);
            match eval_simple_script(&s, &ctx, "() => window.name").await {
                Ok(name) => {
                    f.insert("name".to_string(), json!(name));
                }
                Err(_) => {
                    f.insert("name".to_string(), json!(""));
                }
            }
        }
    }

    /// `browserlane:page.frames` — returns all child frames of a page.
    pub(crate) async fn handle_page_frames(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let mut frames = match self.get_frame_tree(session, &context).await {
            Ok(f) => f,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        self.resolve_frame_names(session, &mut frames).await;

        let frames_val: Vec<Value> = frames.into_iter().map(Value::Object).collect();
        self.send_success(session, cmd.id, json!({ "frames": frames_val }));
    }

    /// `browserlane:page.frame` — finds a frame by name (exact) or URL substring.
    pub(crate) async fn handle_page_frame(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let name_or_url = cmd.params.get("nameOrUrl").and_then(Value::as_str).unwrap_or("");
        if name_or_url.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("nameOrUrl parameter is required"));
        }
        let name_or_url = name_or_url.to_string();

        let mut frames = match self.get_frame_tree(session, &context).await {
            Ok(f) => f,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        self.resolve_frame_names(session, &mut frames).await;

        // Match by name first (exact match).
        for f in &frames {
            if f.get("name").and_then(Value::as_str).unwrap_or("") == name_or_url {
                return self.send_success(session, cmd.id, Value::Object(f.clone()));
            }
        }
        // Then match by URL substring.
        for f in &frames {
            if f.get("url").and_then(Value::as_str).unwrap_or("").contains(&name_or_url) {
                return self.send_success(session, cmd.id, Value::Object(f.clone()));
            }
        }

        // No match found — return null.
        self.send_success(session, cmd.id, Value::Null);
    }
}

// ---------------------------------------------------------------------------
// Exported standalone frame functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Information about a child frame.
#[derive(Debug, Clone, Serialize)]
pub struct FrameInfo {
    pub context: String,
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// Returns all child frames of the given browsing context (with names).
pub async fn list_frames(s: &dyn Session, context: &str) -> anyhow::Result<Vec<FrameInfo>> {
    let resp = s
        .send_bidi_command("browsingContext.getTree", json!({ "root": context }))
        .await?;
    check_bidi_error(&resp)?;

    let parsed: GetTreeResult =
        serde_json::from_value(resp).map_err(|e| anyhow!("failed to parse getTree response: {e}"))?;

    let raw = match parsed.result.contexts.first() {
        Some(root) => collect_frames(root.children.as_deref().unwrap_or(&[])),
        None => Vec::new(),
    };

    let mut frames = Vec::with_capacity(raw.len());
    for f in raw {
        let ctx = f.get("context").and_then(Value::as_str).unwrap_or("").to_string();
        let url = f.get("url").and_then(Value::as_str).unwrap_or("").to_string();
        let mut fi = FrameInfo { context: ctx.clone(), url, name: String::new() };
        // Resolve window.name.
        if let Ok(name) = eval_simple_script(s, &ctx, "() => window.name").await {
            fi.name = name;
        }
        frames.push(fi);
    }
    Ok(frames)
}

/// Finds a child frame by name (exact) or URL substring.
pub async fn find_frame(
    s: &dyn Session,
    context: &str,
    name_or_url: &str,
) -> anyhow::Result<Option<FrameInfo>> {
    let frames = list_frames(s, context).await?;

    // Match by name first (exact match).
    for f in &frames {
        if f.name == name_or_url {
            return Ok(Some(f.clone()));
        }
    }
    // Then match by URL substring.
    for f in &frames {
        if f.url.contains(name_or_url) {
            return Ok(Some(f.clone()));
        }
    }

    Ok(None)
}
