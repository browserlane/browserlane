//! WebSocket monitoring: `browserlane:page.onWebSocket` installs a preload script that
//! wraps `window.WebSocket` and pipes connection/message/close events over a BiDi
//! `script.message` channel; the router intercepts those channel events and
//! re-emits them to the client as `browserlane:ws.*` events.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::{json, Map, Value};

use super::helpers::check_bidi_error;
use super::router::{BidiCommand, BrowserSession, Router};

/// Injected via `script.addPreloadScript` to wrap `window.WebSocket` and observe
/// all WS connections, messages, and close events. Receives a BiDi channel
/// function as its argument.
///
/// Idempotent: bails if already installed. Since the preload applies to every
/// context in the session, a follow-up `script.callFunction` injection on the
/// current page would otherwise double-wrap `window.WebSocket` and emit every WS
/// event twice.
const WS_MONITOR_PRELOAD_SCRIPT: &str = r#"(channel) => {
	if (window.__browserlaneWsMonitorInstalled) return;
	window.__browserlaneWsMonitorInstalled = true;
	const OrigWS = window.WebSocket;
	let nextId = 1;

	window.WebSocket = function(url, protocols) {
		const id = nextId++;
		const urlStr = typeof url === 'string' ? url : url.toString();
		const realWS = protocols !== undefined ? new OrigWS(url, protocols) : new OrigWS(url);

		channel(JSON.stringify({ type: 'created', id: id, url: urlStr }));

		realWS.addEventListener('open', () => {
			channel(JSON.stringify({ type: 'open', id: id }));
		});
		realWS.addEventListener('message', (e) => {
			channel(JSON.stringify({ type: 'message', id: id, data: typeof e.data === 'string' ? e.data : '[binary]', direction: 'received' }));
		});
		realWS.addEventListener('close', (e) => {
			channel(JSON.stringify({ type: 'close', id: id, code: e.code, reason: e.reason }));
		});
		realWS.addEventListener('error', () => {
			channel(JSON.stringify({ type: 'error', id: id }));
		});

		const origSend = realWS.send.bind(realWS);
		realWS.send = function(data) {
			channel(JSON.stringify({ type: 'message', id: id, data: typeof data === 'string' ? data : '[binary]', direction: 'sent' }));
			return origSend(data);
		};

		return realWS;
	};

	window.WebSocket.CONNECTING = 0;
	window.WebSocket.OPEN = 1;
	window.WebSocket.CLOSING = 2;
	window.WebSocket.CLOSED = 3;
	window.WebSocket.prototype = OrigWS.prototype;
}"#;

const WS_CHANNEL_NAME: &str = "browserlane-ws";

impl Router {
    /// Handles `browserlane:page.onWebSocket` — installs the WebSocket monitoring
    /// preload script and subscribes to `script.message` events.
    pub(crate) async fn handle_page_on_web_socket(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        // Subscribe to script.message events (once per session).
        if !session.ws_subscribed.load(Ordering::SeqCst) {
            let resp = match self
                .send_internal_command(session, "session.subscribe", json!({ "events": ["script.message"] }))
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
            session.ws_subscribed.store(true, Ordering::SeqCst);
        }

        // Install preload script (once per session) — applies to all future navigations.
        let need_preload = session.ws_preload_script_id.lock().unwrap().is_empty();
        if need_preload {
            // Omit "contexts" so the preload applies to ALL browsing contexts in the
            // session and re-fires on every navigation.
            let resp = match self
                .send_internal_command(
                    session,
                    "script.addPreloadScript",
                    json!({
                        "functionDeclaration": WS_MONITOR_PRELOAD_SCRIPT,
                        "arguments": [{
                            "type": "channel",
                            "value": { "channel": WS_CHANNEL_NAME },
                        }],
                    }),
                )
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
            let script_id = resp
                .get("result")
                .and_then(|r| r.get("script"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            *session.ws_preload_script_id.lock().unwrap() = script_id;
        }

        // Also inject on the current page (preload only fires on future navigations).
        if let Err(e) = self
            .send_internal_command(
                session,
                "script.callFunction",
                json!({
                    "functionDeclaration": WS_MONITOR_PRELOAD_SCRIPT,
                    "target": { "context": context },
                    "arguments": [{
                        "type": "channel",
                        "value": { "channel": WS_CHANNEL_NAME },
                    }],
                    "awaitPromise": false,
                }),
            )
            .await
        {
            self.send_error(session, cmd.id, &e);
            return;
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Checks if a browser message is a `script.message` from the WS channel. If
    /// so, translates the event and sends it to the client, returning true.
    pub(crate) fn is_ws_channel_event(&self, session: &Arc<BrowserSession>, msg: &str) -> bool {
        if !session.ws_subscribed.load(Ordering::SeqCst) {
            return false;
        }

        let event: Value = match serde_json::from_str(msg) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let method = event.get("method").and_then(Value::as_str).unwrap_or("");
        let params = event.get("params");
        let channel = params.and_then(|p| p.get("channel")).and_then(Value::as_str).unwrap_or("");
        if method != "script.message" || channel != WS_CHANNEL_NAME {
            return false;
        }

        // Parse the data — it's a BiDi remote value wrapping our JSON string.
        let data = params.and_then(|p| p.get("data"));
        let data_type = data.and_then(|d| d.get("type")).and_then(Value::as_str).unwrap_or("");
        if data_type != "string" {
            return false;
        }
        let data_value = data.and_then(|d| d.get("value")).and_then(Value::as_str).unwrap_or("");

        // Parse our WS event JSON (must be an object, like Go's map decode).
        let ws_event: Map<String, Value> = match serde_json::from_str(data_value) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let context = params
            .and_then(|p| p.get("source"))
            .and_then(|s| s.get("context"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        self.translate_ws_event(session, &ws_event, &context);
        true
    }

    /// Converts a raw WS monitor event into a `browserlane:ws.*` event and sends it to
    /// the client.
    fn translate_ws_event(&self, session: &Arc<BrowserSession>, ws_event: &Map<String, Value>, context: &str) {
        let event_type = ws_event.get("type").and_then(Value::as_str).unwrap_or("");

        let mut params = Map::new();
        params.insert("context".to_string(), Value::from(context.to_string()));

        // Copy the id field (JSON number → int).
        if let Some(id) = ws_event.get("id").and_then(Value::as_f64) {
            params.insert("id".to_string(), Value::from(id as i64));
        }

        let s = |k: &str| ws_event.get(k).and_then(Value::as_str).unwrap_or("").to_string();

        let method = match event_type {
            "created" => {
                params.insert("url".to_string(), Value::from(s("url")));
                "browserlane:ws.created"
            }
            "open" => "browserlane:ws.open",
            "message" => {
                params.insert("data".to_string(), Value::from(s("data")));
                params.insert("direction".to_string(), Value::from(s("direction")));
                "browserlane:ws.message"
            }
            "close" => {
                if let Some(code) = ws_event.get("code").and_then(Value::as_f64) {
                    params.insert("code".to_string(), Value::from(code as i64));
                }
                params.insert("reason".to_string(), Value::from(s("reason")));
                "browserlane:ws.closed"
            }
            "error" => "browserlane:ws.error",
            _ => return,
        };

        let event_msg = json!({ "method": method, "params": Value::Object(params) });
        if let Ok(data) = serde_json::to_string(&event_msg) {
            let _ = session.client.send(&data);
        }
    }
}
