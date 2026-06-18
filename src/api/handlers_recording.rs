//! The six `browserlane:recording.*` routes plus the screenshot/snapshot capture
//! helpers shared by the proxy dispatch() and the MCP Call() paths.
//!
//! Faithful-port note: Go's `captureScreenshotForRecording` is the capture
//! callback for `StartScreenshotLoop`, which has no caller anywhere, so both are
//! omitted (screenshots are captured per-action in dispatch).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Map, Value};

use super::helpers::{check_bidi_error, eval_simple_script};
use super::recording::{
    decode_base64, image_dimensions, new_recorder, now_unix_millis, parse_recording_options,
    write_record_to_file, Recorder, RecordingStartOptions,
};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::{new_api_session, Session};

impl Router {
    /// Handles `browserlane:recording.start` — starts recording.
    /// Options: name, screenshots, snapshots, sources, title.
    pub(crate) async fn handle_recording_start(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let opts = parse_recording_options(&cmd.params);

        // Best-effort viewport query
        let viewport = self.query_viewport(session).await;

        // Create and start the recorder
        let recorder = Arc::new(new_recorder());
        recorder.start(opts, viewport);

        *session.recorder.lock().unwrap() = Some(recorder);

        // Screenshots are captured per-action in dispatch(), not via a background loop.

        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:recording.stop` — stops recording and returns recording data.
    /// Options: path (file path to save zip).
    pub(crate) async fn handle_recording_stop(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        // Wait for any in-flight dispatch() to finish so its after-event is recorded.
        let _guard = session.dispatch_mu.lock().await;

        let recorder = session.recorder.lock().unwrap().clone();
        let recorder = match recorder {
            Some(r) => r,
            None => {
                self.send_error(session, cmd.id, &anyhow::anyhow!("recording is not started"));
                return;
            }
        };

        // Stop recording and get zip data
        let zip_data = match recorder.stop() {
            Ok(d) => d,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        // Clear the recorder from the session
        *session.recorder.lock().unwrap() = None;

        // Write to file or return base64
        if let Some(path) = cmd.params.get("path").and_then(Value::as_str).filter(|p| !p.is_empty()) {
            if let Err(e) = write_record_to_file(&zip_data, path) {
                self.send_error(session, cmd.id, &anyhow::anyhow!("failed to write recording: {e}"));
                return;
            }
            self.send_success(session, cmd.id, json!({ "path": path }));
        } else {
            let encoded = base64_encode(&zip_data);
            self.send_success(session, cmd.id, json!({ "data": encoded }));
        }
    }

    /// Handles `browserlane:recording.startChunk` — starts a new recording chunk.
    /// Options: name, title.
    pub(crate) async fn handle_recording_start_chunk(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        // Wait for any in-flight dispatch() to finish so events are properly ordered.
        let _guard = session.dispatch_mu.lock().await;

        let recorder = session.recorder.lock().unwrap().clone();
        let recorder = match recorder {
            Some(r) => r,
            None => {
                self.send_error(session, cmd.id, &anyhow::anyhow!("recording is not started"));
                return;
            }
        };

        let name = cmd.params.get("name").and_then(Value::as_str).unwrap_or("");
        let title = cmd.params.get("title").and_then(Value::as_str).unwrap_or("");

        // Best-effort viewport query
        let viewport = self.query_viewport(session).await;

        recorder.start_chunk(name, title, viewport);
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:recording.stopChunk` — stops the current chunk.
    /// Options: path (file path to save zip).
    pub(crate) async fn handle_recording_stop_chunk(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        // Wait for any in-flight dispatch() to finish so its after-event is recorded.
        let _guard = session.dispatch_mu.lock().await;

        let recorder = session.recorder.lock().unwrap().clone();
        let recorder = match recorder {
            Some(r) => r,
            None => {
                self.send_error(session, cmd.id, &anyhow::anyhow!("recording is not started"));
                return;
            }
        };

        let zip_data = match recorder.stop_chunk() {
            Ok(d) => d,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        if let Some(path) = cmd.params.get("path").and_then(Value::as_str).filter(|p| !p.is_empty()) {
            if let Err(e) = write_record_to_file(&zip_data, path) {
                self.send_error(session, cmd.id, &anyhow::anyhow!("failed to write recording chunk: {e}"));
                return;
            }
            self.send_success(session, cmd.id, json!({ "path": path }));
        } else {
            let encoded = base64_encode(&zip_data);
            self.send_success(session, cmd.id, json!({ "data": encoded }));
        }
    }

    /// Handles `browserlane:recording.startGroup` — starts a named group in the recording.
    pub(crate) async fn handle_recording_start_group(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        // Wait for any in-flight dispatch() to finish so events are properly ordered.
        let _guard = session.dispatch_mu.lock().await;

        let recorder = session.recorder.lock().unwrap().clone();
        let recorder = match recorder {
            Some(r) => r,
            None => {
                self.send_error(session, cmd.id, &anyhow::anyhow!("recording is not started"));
                return;
            }
        };

        let name = cmd.params.get("name").and_then(Value::as_str).unwrap_or("");
        if name.is_empty() {
            self.send_error(session, cmd.id, &anyhow::anyhow!("name is required for recording.startGroup"));
            return;
        }

        recorder.start_group(name);
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:recording.stopGroup` — ends the current group.
    pub(crate) async fn handle_recording_stop_group(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        // Wait for any in-flight dispatch() to finish so its after-event is recorded.
        let _guard = session.dispatch_mu.lock().await;

        let recorder = session.recorder.lock().unwrap().clone();
        let recorder = match recorder {
            Some(r) => r,
            None => {
                self.send_error(session, cmd.id, &anyhow::anyhow!("recording is not started"));
                return;
            }
        };

        recorder.stop_group();
        self.send_success(session, cmd.id, json!({}));
    }

    /// Captures a before-snapshot for click-like actions after the element has
    /// been scrolled into view. Called from interaction handlers between
    /// resolve_with_actionability and the actual input action.
    pub(crate) async fn capture_before_snapshot_after_scroll(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        params: &Map<String, Value>,
    ) {
        let call_id = params.get("_recordCallId").and_then(Value::as_str).unwrap_or("");
        if call_id.is_empty() {
            return;
        }
        let recorder = session.recorder.lock().unwrap().clone();
        let recorder = match recorder {
            Some(r) if r.is_recording() => r,
            _ => return,
        };
        if !recorder.options().snapshots {
            return;
        }
        let name = self.capture_action_snapshot(session, &recorder, params, call_id, "before").await;
        if !name.is_empty() {
            recorder.patch_before_snapshot(call_id, &name);
        }
    }

    /// Captures a screenshot and wraps it as a frame-snapshot for the Record
    /// Player / Playwright trace viewer. Returns the snapshot name (e.g.
    /// "before@call@1") or "" on failure.
    pub(crate) async fn capture_action_snapshot(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        recorder: &Recorder,
        params: &Map<String, Value>,
        call_id: &str,
        snapshot_type: &str,
    ) -> String {
        if session.closed.load(Ordering::SeqCst) {
            return String::new();
        }

        // Resolve browsing context from params or session
        let mut context = params.get("context").and_then(Value::as_str).unwrap_or("").to_string();
        if context.is_empty() {
            context = session.last_context.lock().unwrap().clone();
        }
        if context.is_empty() {
            match self.get_context(session).await {
                Ok(c) => context = c,
                Err(_) => return String::new(),
            }
        }

        // Capture screenshot via native BiDi command (no JS execution)
        let opts = recorder.options();
        let resp = match self
            .send_internal_command_with_timeout(
                session,
                "browsingContext.captureScreenshot",
                screenshot_params(&context, &opts),
                Duration::from_secs(2),
            )
            .await
        {
            Ok(r) => r,
            Err(_) => return String::new(),
        };

        if check_bidi_error(&resp).is_err() {
            return String::new();
        }

        let data = resp
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if data.is_empty() {
            return String::new();
        }

        // Decode image and compute dimensions (handles both PNG and JPEG)
        let img_data = match decode_base64(data) {
            Ok(d) => d,
            Err(_) => return String::new(),
        };
        let (w, h) = image_dimensions(&img_data);

        // Store image in resources for Record Player
        let name = recorder.screenshot_name(&context, now_unix_millis());
        recorder.store_resource(&name, img_data);

        // Inline data URI for Playwright compat (its service worker only intercepts HTTP(S))
        let mime_type = if opts.format == "png" { "image/png" } else { "image/jpeg" };
        let img_src = format!("data:{mime_type};base64,{data}");

        // Build minimal HTML with inline screenshot
        let html = json!([
            "HTML", {},
            ["HEAD", {}],
            [
                "BODY", { "style": "margin:0;overflow:hidden" },
                [
                    "IMG", {
                        "src": img_src,
                        "style": "width:100%",
                    },
                ],
            ],
        ]);

        let viewport = json!({ "width": w, "height": h });

        let resource_overrides = json!([{ "url": img_src, "sha1": name }]);

        let frame_url = session.last_url.lock().unwrap().clone();

        recorder.add_frame_snapshot(
            call_id,
            snapshot_type,
            &context,
            &frame_url,
            "html",
            html,
            viewport,
            resource_overrides,
        )
    }

    /// Queries the browser for the current viewport size. Returns None if the
    /// query fails (best-effort).
    pub(crate) async fn query_viewport(self: &Arc<Self>, session: &Arc<BrowserSession>) -> Option<Value> {
        let context = self.get_context(session).await.ok()?;
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let result = eval_simple_script(&s, &context, "() => window.innerWidth + ',' + window.innerHeight")
            .await
            .ok()?;
        let (w_str, h_str) = result.split_once(',')?;
        let w: i64 = w_str.parse().ok()?;
        let h: i64 = h_str.parse().ok()?;
        Some(json!({ "width": w, "height": h }))
    }
}

/// Builds the BiDi captureScreenshot params with optional format/quality.
pub fn screenshot_params(context: &str, opts: &RecordingStartOptions) -> Value {
    let mut params = Map::new();
    params.insert("context".to_string(), Value::from(context.to_string()));
    if opts.format == "jpeg" {
        let mut f = Map::new();
        f.insert("type".to_string(), Value::from("image/jpeg"));
        if opts.quality > 0.0 {
            f.insert("quality".to_string(), json!(opts.quality));
        }
        params.insert("format".to_string(), Value::Object(f));
    }
    Value::Object(params)
}

/// Captures a screenshot via the Session interface and adds it to the recorder.
/// Shared by both the proxy dispatch() and MCP Call() paths. The Session's
/// get_context_id() handles context resolution (explicit → lastContext → getTree).
pub async fn capture_recording_screenshot(s: &dyn Session, recorder: &Recorder, action_end_unix_millis: i64) {
    if !recorder.options().screenshots {
        return;
    }

    let context = match s.get_context_id().await {
        Ok(c) => c,
        Err(_) => return,
    };

    let opts = recorder.options();
    let resp = match s
        .send_bidi_command_with_timeout(
            "browsingContext.captureScreenshot",
            screenshot_params(&context, &opts),
            Duration::from_secs(5),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    if check_bidi_error(&resp).is_err() {
        return;
    }

    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let img_data = match decode_base64(data) {
        Ok(d) => d,
        Err(_) => return,
    };

    let (w, h) = image_dimensions(&img_data);
    recorder.add_screenshot(img_data, &context, w, h, action_end_unix_millis);
}

/// Standard base64 encoding (Go's base64.StdEncoding.EncodeToString).
fn base64_encode(data: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    STANDARD.encode(data)
}
