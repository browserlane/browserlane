//! Phase 2 ports the standalone navigation functions (shared by proxy + agent)
//! and the `browserlane:page.navigate` router route. The other page routes
//! (back/forward/reload/url/title/content/waitFor*) are ported in Phase 3 when
//! their routes are wired into the dispatch switch.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use serde_json::{json, Value};
use tokio::time::sleep;

use super::handlers_recording::capture_recording_screenshot;
use super::helpers::{check_bidi_error, eval_simple_script};
use super::recording::now_unix_millis;
use super::router::{BidiCommand, BrowserSession, Router, DEFAULT_TIMEOUT};
use super::session::{new_api_session, Session};

// ---------------------------------------------------------------------------
// Router route: browserlane:page.navigate
// ---------------------------------------------------------------------------

impl Router {
    /// Handles `browserlane:page.navigate` — navigates to a URL.
    pub(crate) async fn handle_page_navigate(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        let url = cmd.params.get("url").and_then(Value::as_str).unwrap_or("");
        if url.is_empty() {
            self.send_error(session, cmd.id, &anyhow!("url is required"));
            return;
        }

        let wait = cmd.params.get("wait").and_then(Value::as_str).unwrap_or("");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = navigate(&s, &context, url, wait).await {
            self.send_error(session, cmd.id, &e);
            return;
        }

        // Capture filmstrip screenshot while the page is in its clean post-navigate
        // state, before send_success unblocks the client to send further commands.
        let recorder = session.recorder.lock().unwrap().clone();
        if let Some(rec) = &recorder {
            if rec.is_recording() {
                let ps = new_api_session(Arc::clone(self), Arc::clone(session), &context);
                capture_recording_screenshot(&ps, rec, now_unix_millis()).await;
                session.handler_screenshot.store(true, Ordering::SeqCst);
            }
        }

        self.send_success(session, cmd.id, json!({ "url": url }));
    }

    /// Handles `browserlane:page.back` — navigates back in history.
    pub(crate) async fn handle_page_back(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = go_back(&s, &context).await {
            self.send_error(session, cmd.id, &e);
            return;
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.forward` — navigates forward in history.
    pub(crate) async fn handle_page_forward(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = go_forward(&s, &context).await {
            self.send_error(session, cmd.id, &e);
            return;
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.reload` — reloads the current page.
    pub(crate) async fn handle_page_reload(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let wait = cmd.params.get("wait").and_then(Value::as_str).unwrap_or("");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = reload(&s, &context, wait).await {
            self.send_error(session, cmd.id, &e);
            return;
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.url` — returns the current page URL.
    pub(crate) async fn handle_page_url(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match get_url(&s, &context).await {
            Ok(url) => self.send_success(session, cmd.id, json!({ "url": url })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:page.title` — returns the current page title.
    pub(crate) async fn handle_page_title(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match get_title(&s, &context).await {
            Ok(title) => self.send_success(session, cmd.id, json!({ "title": title })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:page.content` — returns the page's full HTML.
    pub(crate) async fn handle_page_content(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match get_content(&s, &context).await {
            Ok(content) => self.send_success(session, cmd.id, json!({ "content": content })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:page.waitForURL` — waits until the URL matches a pattern.
    pub(crate) async fn handle_page_wait_for_url(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let pattern = cmd.params.get("pattern").and_then(Value::as_str).unwrap_or("");
        if pattern.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("pattern is required"));
        }
        let timeout = param_timeout(&cmd.params);
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match wait_for_url(&s, &context, pattern, timeout).await {
            Ok(url) => self.send_success(session, cmd.id, json!({ "url": url })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:page.waitForLoad` — waits until the page reaches a load state.
    pub(crate) async fn handle_page_wait_for_load(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let state = cmd.params.get("state").and_then(Value::as_str).unwrap_or("");
        let timeout = param_timeout(&cmd.params);
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match wait_for_load(&s, &context, state, timeout).await {
            Ok(()) => self.send_success(session, cmd.id, json!({})),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }
}

/// Extracts a millisecond "timeout" param, defaulting to DEFAULT_TIMEOUT.
fn param_timeout(params: &serde_json::Map<String, Value>) -> Duration {
    match params.get("timeout").and_then(Value::as_f64) {
        Some(ms) if ms > 0.0 => Duration::from_millis(ms as u64),
        _ => DEFAULT_TIMEOUT,
    }
}

// ---------------------------------------------------------------------------
// Exported standalone navigation functions — usable from both proxy and agent.
// ---------------------------------------------------------------------------

/// Navigates to a URL and waits for the given load state.
pub async fn navigate(s: &dyn Session, context: &str, url: &str, wait: &str) -> anyhow::Result<()> {
    let wait = if wait.is_empty() { "complete" } else { wait };

    let params = json!({
        "context": context,
        "url": url,
        "wait": wait,
    });

    let resp = s.send_bidi_command("browsingContext.navigate", params).await?;
    check_bidi_error(&resp)?;
    Ok(())
}

/// Navigates back in history.
pub async fn go_back(s: &dyn Session, context: &str) -> anyhow::Result<()> {
    let params = json!({ "context": context, "delta": -1 });
    let resp = s
        .send_bidi_command("browsingContext.traverseHistory", params)
        .await?;
    check_bidi_error(&resp)?;
    let _ = wait_for_ready_state(s, context, "complete", Duration::from_secs(10)).await;
    Ok(())
}

/// Navigates forward in history.
pub async fn go_forward(s: &dyn Session, context: &str) -> anyhow::Result<()> {
    let params = json!({ "context": context, "delta": 1 });
    let resp = s
        .send_bidi_command("browsingContext.traverseHistory", params)
        .await?;
    check_bidi_error(&resp)?;
    let _ = wait_for_ready_state(s, context, "complete", Duration::from_secs(10)).await;
    Ok(())
}

/// Reloads the current page and waits for the given load state.
pub async fn reload(s: &dyn Session, context: &str, wait: &str) -> anyhow::Result<()> {
    let wait = if wait.is_empty() { "complete" } else { wait };
    let params = json!({ "context": context, "wait": wait });
    let resp = s.send_bidi_command("browsingContext.reload", params).await?;
    check_bidi_error(&resp)?;
    Ok(())
}

/// Returns the current page URL.
pub async fn get_url(s: &dyn Session, context: &str) -> anyhow::Result<String> {
    eval_simple_script(s, context, "() => window.location.href").await
}

/// Returns the current page title.
pub async fn get_title(s: &dyn Session, context: &str) -> anyhow::Result<String> {
    eval_simple_script(s, context, "() => document.title").await
}

/// Returns the page's full HTML.
pub async fn get_content(s: &dyn Session, context: &str) -> anyhow::Result<String> {
    eval_simple_script(s, context, "() => document.documentElement.outerHTML").await
}

/// Waits until the URL matches a pattern.
pub async fn wait_for_url(
    s: &dyn Session,
    context: &str,
    pattern: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(url) = eval_simple_script(s, context, "() => window.location.href").await {
            if matches_pattern(&url, pattern) {
                return Ok(url);
            }
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!(
                "timeout after {} waiting for URL matching '{}'",
                crate::errors::format_go_duration(timeout),
                pattern
            ));
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// Waits until the page reaches a given load state.
pub async fn wait_for_load(
    s: &dyn Session,
    context: &str,
    state: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    let state = if state.is_empty() { "complete" } else { state };
    wait_for_ready_state(s, context, state, timeout).await
}

/// Polls document.readyState until it matches the target state.
pub async fn wait_for_ready_state(
    s: &dyn Session,
    context: &str,
    target_state: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(state) = eval_simple_script(s, context, "() => document.readyState").await {
            if ready_state_reached(&state, target_state) {
                return Ok(());
            }
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!(
                "timeout after {} waiting for readyState '{}'",
                crate::errors::format_go_duration(timeout),
                target_state
            ));
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// Checks if the current readyState meets or exceeds the target.
/// Order: loading < interactive < complete.
fn ready_state_reached(current: &str, target: &str) -> bool {
    let rank = |s: &str| match s {
        "loading" => Some(0),
        "interactive" => Some(1),
        "complete" => Some(2),
        _ => None,
    };
    match (rank(current), rank(target)) {
        (Some(c), Some(t)) => c >= t,
        _ => current == target,
    }
}

/// Checks if a URL matches a pattern (exact, glob with `*`, or substring).
fn matches_pattern(url: &str, pattern: &str) -> bool {
    if url == pattern {
        return true;
    }
    if pattern.contains('*') {
        return glob_match(url, pattern);
    }
    url.contains(pattern)
}

/// Simple glob matching where `*` matches any characters.
fn glob_match(s: &str, pattern: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();

    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match s[pos..].find(part) {
            Some(idx) => {
                if i == 0 && idx != 0 {
                    return false;
                }
                pos += idx + part.len();
            }
            None => return false,
        }
    }

    let last_part = parts[parts.len() - 1];
    if !last_part.is_empty() && !s.ends_with(last_part) {
        return false;
    }

    true
}
