//! Phase 3 network cluster: the request-interception routes (page.route/unroute,
//! network.continue/fulfill/abort, page.setHeaders). The onRequest/onResponse and
//! log.entryAdded events are already subscribed + forwarded by the router's event
//! loop; response.body() is client-side (onResponse + page.eval), so no extra
//! server route is needed for those.

use std::sync::Arc;

use anyhow::anyhow;
use serde_json::{json, Map, Value};

use super::helpers::check_bidi_error;
use super::router::{BidiCommand, BrowserSession, Router};

impl Router {
    /// `browserlane:page.route` — adds a network intercept for beforeRequestSent.
    /// The JS client uses the returned intercept ID to match requests against URL patterns.
    pub(crate) async fn handle_page_route(
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
            "phases": ["beforeRequestSent"],
            "contexts": [context],
        });

        let resp = match self
            .send_internal_command(session, "network.addIntercept", params)
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

        let intercept = resp
            .get("result")
            .and_then(|r| r.get("intercept"))
            .and_then(Value::as_str)
            .unwrap_or("");
        self.send_success(session, cmd.id, json!({ "intercept": intercept }));
    }

    /// `browserlane:page.unroute` — removes a network intercept.
    pub(crate) async fn handle_page_unroute(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let intercept = cmd.params.get("intercept").and_then(Value::as_str).unwrap_or("");
        if intercept.is_empty() {
            self.send_error(session, cmd.id, &anyhow!("intercept is required"));
            return;
        }

        let params = json!({ "intercept": intercept });
        let resp = match self
            .send_internal_command(session, "network.removeIntercept", params)
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

    /// `browserlane:network.continue` — continues an intercepted request.
    /// Optional overrides: url, method, headers, body.
    pub(crate) async fn handle_network_continue(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let request = cmd.params.get("request").and_then(Value::as_str).unwrap_or("");
        if request.is_empty() {
            self.send_error(session, cmd.id, &anyhow!("request is required"));
            return;
        }

        let mut params = json!({ "request": request });
        if let Some(url) = cmd.params.get("url").and_then(Value::as_str) {
            if !url.is_empty() {
                params["url"] = json!(url);
            }
        }
        if let Some(method) = cmd.params.get("method").and_then(Value::as_str) {
            if !method.is_empty() {
                params["method"] = json!(method);
            }
        }
        if let Some(body) = cmd.params.get("body").and_then(Value::as_str) {
            params["body"] = json!({ "type": "string", "value": body });
        }
        if let Some(headers) = cmd.params.get("headers").and_then(Value::as_object) {
            params["headers"] = Value::Array(convert_headers_to_bidi(headers));
        }

        let resp = match self
            .send_internal_command(session, "network.continueRequest", params)
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

    /// `browserlane:network.fulfill` — provides a response for an intercepted request.
    /// Optional: statusCode, headers, body, reasonPhrase, contentType.
    pub(crate) async fn handle_network_fulfill(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let request = cmd.params.get("request").and_then(Value::as_str).unwrap_or("");
        if request.is_empty() {
            self.send_error(session, cmd.id, &anyhow!("request is required"));
            return;
        }

        let mut params = json!({ "request": request });
        if let Some(status) = cmd.params.get("statusCode").and_then(Value::as_f64) {
            params["statusCode"] = json!(status as i64);
        }
        if let Some(reason) = cmd.params.get("reasonPhrase").and_then(Value::as_str) {
            if !reason.is_empty() {
                params["reasonPhrase"] = json!(reason);
            }
        }
        if let Some(body) = cmd.params.get("body").and_then(Value::as_str) {
            params["body"] = json!({ "type": "string", "value": body });
        }

        // Build the headers map (clone so we can inject Content-Type from contentType).
        let mut headers: Map<String, Value> = cmd
            .params
            .get("headers")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        // contentType convenience: inject Content-Type if not already present.
        if let Some(content_type) = cmd.params.get("contentType").and_then(Value::as_str) {
            if !content_type.is_empty() {
                let has_content_type = headers
                    .keys()
                    .any(|name| name.eq_ignore_ascii_case("content-type"));
                if !has_content_type {
                    headers.insert("Content-Type".to_string(), json!(content_type));
                }
            }
        }

        if !headers.is_empty() {
            params["headers"] = Value::Array(convert_headers_to_bidi(&headers));
        }

        let resp = match self
            .send_internal_command(session, "network.provideResponse", params)
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

    /// `browserlane:network.abort` — fails an intercepted request.
    pub(crate) async fn handle_network_abort(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let request = cmd.params.get("request").and_then(Value::as_str).unwrap_or("");
        if request.is_empty() {
            self.send_error(session, cmd.id, &anyhow!("request is required"));
            return;
        }

        let params = json!({ "request": request });
        let resp = match self
            .send_internal_command(session, "network.failRequest", params)
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

    /// `browserlane:page.setHeaders` — sets extra HTTP headers for a context via an intercept.
    pub(crate) async fn handle_page_set_headers(
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

        let headers = match cmd.params.get("headers").and_then(Value::as_object) {
            Some(h) => h.clone(),
            None => {
                self.send_error(session, cmd.id, &anyhow!("headers is required"));
                return;
            }
        };

        let intercept_params = json!({
            "phases": ["beforeRequestSent"],
            "contexts": [context],
        });

        let resp = match self
            .send_internal_command(session, "network.addIntercept", intercept_params)
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

        let intercept = resp
            .get("result")
            .and_then(|r| r.get("intercept"))
            .and_then(Value::as_str)
            .unwrap_or("");

        let bidi_headers = convert_headers_to_bidi(&headers);
        self.send_success(
            session,
            cmd.id,
            json!({
                "intercept": intercept,
                "headers": bidi_headers,
            }),
        );
    }
}

/// Converts headers from `{"Name": "Value"}` to BiDi format:
/// `[{name: "Name", value: {type: "string", value: "Value"}}]`.
fn convert_headers_to_bidi(headers: &Map<String, Value>) -> Vec<Value> {
    headers
        .iter()
        .map(|(name, val)| {
            let val_str = val.as_str().unwrap_or("");
            json!({
                "name": name,
                "value": { "type": "string", "value": val_str },
            })
        })
        .collect()
}
