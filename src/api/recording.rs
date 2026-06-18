//! The trace/filmstrip/DOM-snapshot "record-player" engine: collects events,
//! screenshots, and DOM snapshots, then packages them into a Playwright-
//! compatible trace zip.
//!
//! Faithful-port notes:
//! - Go's `StartScreenshotLoop` has no caller anywhere (screenshots are captured
//!   per-action in dispatch), so it is omitted; `stop_screenshots` is kept as a
//!   no-op for call-site fidelity (closeSession / browserRecordStop).
//! - Whole-valued quantities (monotonic times, version, width/height) are stored
//!   as integer JSON Values so they render like Go's `json.Marshal(float64(N))`
//!   (e.g. `0`, `123`); box/point coords stay f64 (may be fractional).

use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde_json::{json, Map, Value};

use super::helpers::BoxInfo;

/// A generic recording event, stored as a JSON-friendly object (Go's recordEvent).
type RecordEvent = Map<String, Value>;

/// Configures how recording behaves.
#[derive(Debug, Clone, Default)]
pub struct RecordingStartOptions {
    pub name: String,
    pub screenshots: bool,
    pub snapshots: bool,
    pub sources: bool,
    pub title: String,
    pub bidi: bool,
    /// "png" or "jpeg" (default "jpeg").
    pub format: String,
    /// 0.0-1.0 for JPEG (default 0.5).
    pub quality: f64,
}

/// Extracts RecordingStartOptions from a params map. Used by both the proxy
/// (handle_recording_start) and MCP (browser_record_start) paths so option
/// parsing is defined once.
pub fn parse_recording_options(params: &Map<String, Value>) -> RecordingStartOptions {
    let mut opts = RecordingStartOptions {
        screenshots: true, // default: screenshots on (opt out with screenshots=false)
        ..Default::default()
    };
    if let Some(name) = params.get("name").and_then(Value::as_str) {
        opts.name = name.to_string();
    }
    if let Some(title) = params.get("title").and_then(Value::as_str) {
        opts.title = title.to_string();
    }
    if let Some(ss) = params.get("screenshots").and_then(Value::as_bool) {
        opts.screenshots = ss;
    }
    if let Some(sn) = params.get("snapshots").and_then(Value::as_bool) {
        opts.snapshots = sn;
    }
    if let Some(src) = params.get("sources").and_then(Value::as_bool) {
        opts.sources = src;
    }
    if let Some(b) = params.get("bidi").and_then(Value::as_bool) {
        opts.bidi = b;
    }
    // Screenshot format: "jpeg" (default) or "png"
    opts.format = "jpeg".to_string();
    if let Some(f) = params.get("format").and_then(Value::as_str) {
        if f == "png" || f == "jpeg" {
            opts.format = f.to_string();
        }
    }
    opts.quality = 0.5;
    if let Some(q) = params.get("quality").and_then(Value::as_f64) {
        if (0.0..=1.0).contains(&q) {
            opts.quality = q;
        }
    }
    opts
}

/// Tracks a group's name and callId so StopGroup can emit a matching "after" event.
#[derive(Debug, Clone)]
struct GroupEntry {
    name: String,
    call_id: String,
}

/// Holds a parsed beforeRequestSent event until its response arrives.
#[derive(Debug, Clone, Default)]
struct PendingRequest {
    context: String,
    #[allow(dead_code)]
    request_id: String,
    url: String,
    method: String,
    headers: Vec<Value>,
    cookies: Vec<Value>,
    headers_size: f64,
    body_size: f64,
    timestamp: f64, // BiDi timestamp (ms since epoch)
}

/// Mutable recorder state, guarded by the Recorder's mutex (Go's `sync.Mutex`).
struct RecorderInner {
    recording: bool,
    options: RecordingStartOptions,
    events: Vec<RecordEvent>,           // current chunk's recording events
    network: Vec<RecordEvent>,          // current chunk's network events
    resources: HashMap<String, Vec<u8>>, // resource name -> binary data (JPEG/PNG)
    group_stack: Vec<GroupEntry>,       // nested group entries (name + callId)
    pending_requests: HashMap<String, PendingRequest>, // BiDi request ID -> pending request
    chunk_index: i64,
    start_time: i64,    // unix ms
    monotonic_base: i64, // unix ms at recording start; monotonic times are relative to this
    context_id: String, // unique context ID for this recording session
    action_counter: i64, // monotonic counter for action/bidi callIds
}

/// Manages recording state for a browser session. Collects events, screenshots,
/// and DOM snapshots, then packages them into a Playwright-compatible trace zip.
pub struct Recorder {
    inner: Mutex<RecorderInner>,
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a new recorder.
pub fn new_recorder() -> Recorder {
    Recorder::new()
}

impl Recorder {
    /// Creates a new recorder.
    pub fn new() -> Self {
        Recorder {
            inner: Mutex::new(RecorderInner {
                recording: false,
                options: RecordingStartOptions::default(),
                events: Vec::new(),
                network: Vec::new(),
                resources: HashMap::new(),
                group_stack: Vec::new(),
                pending_requests: HashMap::new(),
                chunk_index: 0,
                start_time: 0,
                monotonic_base: 0,
                context_id: String::new(),
                action_counter: 0,
            }),
        }
    }

    /// Returns whether recording is currently active.
    pub fn is_recording(&self) -> bool {
        self.inner.lock().unwrap().recording
    }

    /// Begins recording with the given options. `viewport` is the browser
    /// viewport size (may be None if unknown).
    pub fn start(&self, opts: RecordingStartOptions, viewport: Option<Value>) {
        let mut t = self.inner.lock().unwrap();

        t.recording = true;
        t.options = opts;
        t.events = Vec::new();
        t.network = Vec::new();
        t.resources = HashMap::new();
        t.pending_requests = HashMap::new();
        t.group_stack = Vec::new();
        t.chunk_index = 0;
        t.start_time = now_unix_millis();
        t.monotonic_base = t.start_time;
        t.context_id = format!("context@{:x}", t.start_time);

        let mut title = t.options.title.clone();
        if title.is_empty() {
            title = t.options.name.clone();
        }

        // Build options map
        let mut options = Map::new();
        if let Some(vp) = viewport {
            options.insert("viewport".to_string(), vp);
        }

        let start_time = t.start_time;
        let context_id = t.context_id.clone();
        // First event must be context-options (required by Playwright trace viewer)
        let ev = json!({
            "type": "context-options",
            "browserName": "chromium",
            "platform": go_goos(),
            "wallTime": start_time,
            "monotonicTime": 0,
            "title": title,
            "contextId": context_id,
            "options": Value::Object(options),
            "sdkLanguage": "javascript",
            "version": 8,
            "origin": "library",
            "libraryName": "browserlane",
            "libraryVersion": crate::VERSION,
        });
        t.events.push(obj(ev));
    }

    /// Stops recording and returns the recording zip data.
    pub fn stop(&self) -> anyhow::Result<Vec<u8>> {
        let mut t = self.inner.lock().unwrap();
        if !t.recording {
            return Err(anyhow::anyhow!("recording is not started"));
        }
        t.recording = false;
        t.build_zip()
    }

    /// Starts a new chunk within the current recording. `viewport` may be None.
    pub fn start_chunk(&self, name: &str, title: &str, viewport: Option<Value>) {
        let mut t = self.inner.lock().unwrap();

        t.events = Vec::new();
        t.network = Vec::new();
        t.chunk_index += 1;
        t.monotonic_base = now_unix_millis();

        let mut chunk_title = title.to_string();
        if chunk_title.is_empty() {
            chunk_title = name.to_string();
        }

        let mut options = Map::new();
        if let Some(vp) = viewport {
            options.insert("viewport".to_string(), vp);
        }

        let monotonic_base = t.monotonic_base;
        let context_id = t.context_id.clone();
        let ev = json!({
            "type": "context-options",
            "browserName": "chromium",
            "platform": go_goos(),
            "wallTime": monotonic_base,
            "monotonicTime": 0,
            "title": chunk_title,
            "contextId": context_id,
            "options": Value::Object(options),
            "sdkLanguage": "javascript",
            "version": 8,
            "origin": "library",
            "libraryName": "browserlane",
            "libraryVersion": crate::VERSION,
        });
        t.events.push(obj(ev));
    }

    /// Packages the current chunk into a zip and returns it. Recording remains
    /// active for additional chunks.
    pub fn stop_chunk(&self) -> anyhow::Result<Vec<u8>> {
        let t = self.inner.lock().unwrap();
        if !t.recording {
            return Err(anyhow::anyhow!("recording is not started"));
        }
        t.build_zip()
    }

    /// Adds a group-start marker to the recording.
    pub fn start_group(&self, name: &str) {
        let mut t = self.inner.lock().unwrap();

        let parent_id = t.current_group_id();

        t.action_counter += 1;
        let call_id = format!("call@{}", t.action_counter);
        t.group_stack.push(GroupEntry {
            name: name.to_string(),
            call_id: call_id.clone(),
        });
        let start_time = t.monotonic_now();
        let mut ev = obj(json!({
            "type": "before",
            "callId": call_id,
            "title": name,
            "class": "Tracing",
            "method": "tracingGroup",
            "params": { "name": name },
            "startTime": start_time,
        }));
        if !parent_id.is_empty() {
            ev.insert("parentId".to_string(), Value::from(parent_id));
        }
        t.events.push(ev);
    }

    /// Adds a group-end marker to the recording.
    pub fn stop_group(&self) {
        let mut t = self.inner.lock().unwrap();

        if t.group_stack.is_empty() {
            return;
        }
        let entry = t.group_stack.pop().unwrap();
        let end_time = t.monotonic_now();
        t.events.push(obj(json!({
            "type": "after",
            "callId": entry.call_id,
            "endTime": end_time,
        })));
    }

    /// Returns the current recording options.
    pub fn options(&self) -> RecordingStartOptions {
        self.inner.lock().unwrap().options.clone()
    }

    /// Stores binary data (e.g. screenshot JPEG/PNG) in the resources map, keyed
    /// by name. The data will be written to `resources/<name>` in the zip.
    pub fn store_resource(&self, name: &str, data: Vec<u8>) {
        self.inner
            .lock()
            .unwrap()
            .resources
            .insert(name.to_string(), data);
    }

    /// Generates a Playwright-compatible resource name for a screenshot.
    /// Format: `page@<lowercase-hex>-<wallTimeMs>.<ext>`.
    pub fn screenshot_name(&self, page_id: &str, ts_unix_millis: i64) -> String {
        let t = self.inner.lock().unwrap();
        format!(
            "{}-{}.{}",
            format_page_id(page_id),
            ts_unix_millis,
            t.image_extension()
        )
    }

    /// Generates and returns the next call@N id without emitting any event.
    /// Use when you need the callId before recording the action (e.g. snapshots).
    pub fn next_call_id(&self) -> String {
        let mut t = self.inner.lock().unwrap();
        if !t.recording {
            return String::new();
        }
        t.action_counter += 1;
        format!("call@{}", t.action_counter)
    }

    /// Retroactively adds a beforeSnapshot to an already-emitted "before" event.
    /// Used by click-like handlers that capture the snapshot after scrolling the
    /// element into view but before the actual click/hover/tap action.
    pub fn patch_before_snapshot(&self, call_id: &str, snapshot_name: &str) {
        let mut t = self.inner.lock().unwrap();
        for i in (0..t.events.len()).rev() {
            if t.events[i].get("callId").and_then(Value::as_str) == Some(call_id)
                && t.events[i].get("type").and_then(Value::as_str) == Some("before")
            {
                t.events[i].insert(
                    "beforeSnapshot".to_string(),
                    Value::from(snapshot_name.to_string()),
                );
                return;
            }
        }
    }

    /// Records a browserlane command as an action marker in the recording. `call_id`
    /// should come from next_call_id(). `before_snapshot` is the snapshot name to
    /// link, or "" if none. `page_id` is a fallback browsing context to use when
    /// params["context"] is not set.
    pub fn record_action(
        &self,
        call_id: &str,
        method: &str,
        params: &Map<String, Value>,
        before_snapshot: &str,
        page_id: &str,
    ) {
        let mut t = self.inner.lock().unwrap();

        if !t.recording || call_id.is_empty() {
            return;
        }

        // Shallow-copy params and lowercase context for recording (don't mutate caller's map)
        let mut record_params = params.clone();
        if let Some(ctx) = record_params.get("context").and_then(Value::as_str) {
            if !ctx.is_empty() {
                let lc = ctx.to_lowercase();
                record_params.insert("context".to_string(), Value::from(lc));
            }
        }

        let (class, title) = api_name_from_method(method);
        let start_time = t.monotonic_now();
        let mut ev = obj(json!({
            "type": "before",
            "callId": call_id,
            "title": title,
            "class": class,
            "method": method,
            "startTime": start_time,
        }));
        // Add pageId so the viewer can match actions to page screenshots
        let ctx = record_params
            .get("context")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if !ctx.is_empty() {
            ev.insert("pageId".to_string(), Value::from(format_page_id(&ctx)));
        } else if !page_id.is_empty() {
            ev.insert("pageId".to_string(), Value::from(format_page_id(page_id)));
        }
        ev.insert("params".to_string(), Value::Object(record_params));
        if !before_snapshot.is_empty() {
            ev.insert(
                "beforeSnapshot".to_string(),
                Value::from(before_snapshot.to_string()),
            );
        }
        // Link to parent group for nesting in Record Player
        let gid = t.current_group_id();
        if !gid.is_empty() {
            ev.insert("parentId".to_string(), Value::from(gid));
        }
        t.events.push(ev);
    }

    /// Records the end of a browserlane command action. `call_id` must match the value
    /// returned by next_call_id(). `after_snapshot` is the snapshot name to link,
    /// or "" if none. `end_time_unix_millis` is the actual handler completion time.
    /// `box_` is the bounding box of the interacted element, or None.
    pub fn record_action_end(
        &self,
        call_id: &str,
        after_snapshot: &str,
        end_time_unix_millis: i64,
        box_: Option<BoxInfo>,
    ) {
        let mut t = self.inner.lock().unwrap();

        if !t.recording || call_id.is_empty() {
            return;
        }

        // Emit a Playwright-compatible "input" event with point and box when an
        // element was resolved. The trace viewer reads point from this event type
        // (keyed by callId) to render click-dot overlays.
        if let Some(b) = box_ {
            t.events.push(obj(json!({
                "type": "input",
                "callId": call_id,
                "point": {
                    "x": b.x + b.width / 2.0,
                    "y": b.y + b.height / 2.0,
                },
                "box": {
                    "x": b.x, "y": b.y, "width": b.width, "height": b.height,
                },
            })));
        }

        let end_time = end_time_unix_millis - t.monotonic_base;
        let mut ev = obj(json!({
            "type": "after",
            "callId": call_id,
            "endTime": end_time,
        }));
        if !after_snapshot.is_empty() {
            ev.insert(
                "afterSnapshot".to_string(),
                Value::from(after_snapshot.to_string()),
            );
        }
        t.events.push(ev);
    }

    /// Records a raw BiDi command sent to the browser (opt-in via bidi: true).
    /// Returns the callId so the caller can pass it to record_bidi_command_end.
    pub fn record_bidi_command(&self, method: &str, params: &Map<String, Value>) -> String {
        let mut t = self.inner.lock().unwrap();

        if !t.recording {
            return String::new();
        }

        let mut record_params = params.clone();
        if let Some(ctx) = record_params.get("context").and_then(Value::as_str) {
            if !ctx.is_empty() {
                let lc = ctx.to_lowercase();
                record_params.insert("context".to_string(), Value::from(lc));
            }
        }

        t.action_counter += 1;
        let call_id = format!("call@{}", t.action_counter);
        let start_time = t.monotonic_now();
        let mut ev = obj(json!({
            "type": "before",
            "callId": call_id,
            "title": method,
            "class": "BiDi",
            "method": method,
            "startTime": start_time,
        }));
        ev.insert("params".to_string(), Value::Object(record_params));
        let gid = t.current_group_id();
        if !gid.is_empty() {
            ev.insert("parentId".to_string(), Value::from(gid));
        }
        t.events.push(ev);
        call_id
    }

    /// Records the end of a BiDi command. `call_id` must match the corresponding
    /// record_bidi_command call.
    pub fn record_bidi_command_end(&self, call_id: &str) {
        let mut t = self.inner.lock().unwrap();
        if !t.recording || call_id.is_empty() {
            return;
        }
        let end_time = t.monotonic_now();
        t.events.push(obj(json!({
            "type": "after",
            "callId": call_id,
            "endTime": end_time,
        })));
    }

    /// Stores a screenshot image (PNG or JPEG) and adds a screencast-frame event.
    /// If `ts_unix_millis` is non-zero it is used as the event timestamp; otherwise
    /// the current time is used.
    pub fn add_screenshot(
        &self,
        img_data: Vec<u8>,
        page_id: &str,
        width: i64,
        height: i64,
        ts_unix_millis: i64,
    ) {
        let mut t = self.inner.lock().unwrap();

        if !t.recording {
            return;
        }

        let ts = if ts_unix_millis == 0 {
            now_unix_millis()
        } else {
            ts_unix_millis
        };

        let formatted_page_id = format_page_id(page_id);
        let name = format!("{}-{}.{}", formatted_page_id, ts, t.image_extension());
        t.resources.insert(name.clone(), img_data);
        let timestamp = ts - t.monotonic_base;
        t.events.push(obj(json!({
            "type": "screencast-frame",
            "pageId": formatted_page_id,
            "sha1": name,
            "width": width,
            "height": height,
            "timestamp": timestamp,
        })));
    }

    /// Adds a frame-snapshot event for the Record Player / Playwright trace viewer.
    /// `snapshot_type` is "before" or "after"; `call_id` is like "call@1".
    /// `resource_overrides` maps synthetic URLs to resource SHA1 hashes so the
    /// viewer can resolve them from the zip's resources/ directory. Returns the
    /// snapshot name (e.g. "before@call@1").
    #[allow(clippy::too_many_arguments)]
    pub fn add_frame_snapshot(
        &self,
        call_id: &str,
        snapshot_type: &str,
        page_id: &str,
        frame_url: &str,
        doctype: &str,
        html: Value,
        viewport: Value,
        resource_overrides: Value,
    ) -> String {
        let mut t = self.inner.lock().unwrap();

        if !t.recording {
            return String::new();
        }

        let resource_overrides = if resource_overrides.is_null() {
            Value::Array(Vec::new())
        } else {
            resource_overrides
        };

        let snapshot_name = format!("{snapshot_type}@{call_id}");
        let now = t.monotonic_now();

        let formatted_page_id = format_page_id(page_id);
        t.events.push(obj(json!({
            "type": "frame-snapshot",
            "snapshot": {
                "callId": call_id,
                "snapshotName": snapshot_name,
                "pageId": formatted_page_id,
                "frameId": formatted_page_id,
                "frameUrl": frame_url,
                "doctype": doctype,
                "html": html,
                "viewport": viewport,
                "timestamp": now,
                "wallTime": now,
                "resourceOverrides": resource_overrides,
                "isMainFrame": true,
            },
        })));

        snapshot_name
    }

    /// Records a raw BiDi event from the browser. Network events are correlated by
    /// request ID and transformed into Playwright-compatible HAR resource-snapshot
    /// entries.
    pub fn record_bidi_event(&self, msg: &str) {
        let mut t = self.inner.lock().unwrap();

        if !t.recording {
            return;
        }

        let parsed: Value = match serde_json::from_str(msg) {
            Ok(v) => v,
            Err(_) => return,
        };

        let method = parsed.get("method").and_then(Value::as_str).unwrap_or("");
        // Only record events (not responses)
        if method.is_empty() {
            return;
        }

        let mut params = parsed
            .get("params")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        match method {
            "network.beforeRequestSent" => {
                if let Some(req) = parse_pending_request(&params) {
                    t.pending_requests.insert(req.request_id.clone(), req);
                }
            }
            "network.responseCompleted" => {
                let request_id = extract_request_id(&params);
                let pending = match t.pending_requests.remove(&request_id) {
                    Some(p) => Some(p),
                    None => parse_pending_request_from_response(&params),
                };
                if let Some(pending) = pending {
                    let entry = bidi_to_har_entry(&pending, &params, false, t.monotonic_base);
                    t.network.push(entry);
                }
            }
            "network.fetchError" => {
                let request_id = extract_request_id(&params);
                let pending = match t.pending_requests.remove(&request_id) {
                    Some(p) => Some(p),
                    None => parse_pending_request_from_response(&params),
                };
                if let Some(pending) = pending {
                    let entry = bidi_to_har_entry(&pending, &params, true, t.monotonic_base);
                    t.network.push(entry);
                }
            }
            _ => {
                // Lowercase context in params for consistency
                if let Some(ctx) = params.get("context").and_then(Value::as_str) {
                    if !ctx.is_empty() {
                        let lc = ctx.to_lowercase();
                        params.insert("context".to_string(), Value::from(lc));
                    }
                }
                let time = t.monotonic_now();
                t.events.push(obj(json!({
                    "type": "event",
                    "method": method,
                    "params": Value::Object(params),
                    "time": time,
                    "class": "BrowserContext",
                })));
            }
        }
    }

    /// Mirrors Go's StopScreenshots. The screenshot loop (StartScreenshotLoop) is
    /// never started in any ported code path, so this is a no-op; kept for
    /// call-site fidelity (closeSession / browserRecordStop).
    pub fn stop_screenshots(&self) {}
}

impl RecorderInner {
    /// Returns the current time as relative monotonic ms since recording start.
    /// Always whole (UnixMilli difference), so returned as i64.
    fn monotonic_now(&self) -> i64 {
        now_unix_millis() - self.monotonic_base
    }

    /// Returns the callId of the innermost active group, or "".
    fn current_group_id(&self) -> String {
        match self.group_stack.last() {
            Some(g) => g.call_id.clone(),
            None => String::new(),
        }
    }

    /// Returns the file extension for the recording's image format.
    fn image_extension(&self) -> &'static str {
        if self.options.format == "png" {
            "png"
        } else {
            "jpeg"
        }
    }

    /// Creates the Playwright-compatible recording zip.
    fn build_zip(&self) -> anyhow::Result<Vec<u8>> {
        use zip::write::SimpleFileOptions;
        use zip::CompressionMethod;

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

            // Write trace events
            let trace_name = if self.chunk_index == 0 {
                "trace.trace".to_string()
            } else {
                format!("{}.trace", self.chunk_index)
            };
            zw.start_file(trace_name, opts)
                .map_err(|e| anyhow::anyhow!("failed to create trace entry: {e}"))?;
            for event in &self.events {
                let data = marshal_event(event);
                zw.write_all(data.as_bytes())?;
                zw.write_all(b"\n")?;
            }

            // Write network events
            let net_name = if self.chunk_index == 0 {
                "trace.network".to_string()
            } else {
                format!("{}.network", self.chunk_index)
            };
            zw.start_file(net_name, opts)
                .map_err(|e| anyhow::anyhow!("failed to create network entry: {e}"))?;
            for event in &self.network {
                let data = marshal_event(event);
                zw.write_all(data.as_bytes())?;
                zw.write_all(b"\n")?;
            }

            // Write resources: resources/<name>
            for (name, data) in &self.resources {
                if zw.start_file(format!("resources/{name}"), opts).is_err() {
                    continue;
                }
                zw.write_all(data)?;
            }

            zw.finish()
                .map_err(|e| anyhow::anyhow!("failed to close zip: {e}"))?;
        }
        Ok(cursor.into_inner())
    }
}

/// Maps a browserlane: method to (class, title) for recording display.
fn api_name_from_method(method: &str) -> (String, String) {
    // Strip the "browserlane:" prefix
    if method.len() <= 12 || &method[..12] != "browserlane:" {
        return ("browserlane".to_string(), method.to_string());
    }
    let name = &method[12..]; // e.g. "element.click", "page.navigate"

    let prefixed = |class: &str, prefix_len: usize| {
        (class.to_string(), format!("{}.{}", class, &name[prefix_len..]))
    };

    if name.len() > 8 && &name[..8] == "element." {
        prefixed("Element", 8)
    } else if name.len() > 5 && &name[..5] == "page." {
        prefixed("Page", 5)
    } else if name.len() > 8 && &name[..8] == "browser." {
        prefixed("Browser", 8)
    } else if name.len() > 8 && &name[..8] == "context." {
        ("BrowserContext".to_string(), format!("BrowserContext.{}", &name[8..]))
    } else if (name.len() > 9 && &name[..9] == "keyboard.")
        || (name.len() > 6 && &name[..6] == "mouse.")
        || (name.len() > 6 && &name[..6] == "touch.")
    {
        // keyboard.* / mouse.* / touch.* all map to Page.<full name>
        ("Page".to_string(), format!("Page.{name}"))
    } else if name.len() > 8 && &name[..8] == "network." {
        prefixed("Network", 8)
    } else if name.len() > 7 && &name[..7] == "dialog." {
        prefixed("Dialog", 7)
    } else if name.len() > 6 && &name[..6] == "clock." {
        prefixed("Clock", 6)
    } else if name.len() > 9 && &name[..9] == "download." {
        prefixed("Download", 9)
    } else {
        ("browserlane".to_string(), name.to_string())
    }
}

/// Pulls params.request.request from a BiDi network event.
fn extract_request_id(params: &Map<String, Value>) -> String {
    params
        .get("request")
        .and_then(Value::as_object)
        .and_then(|req| req.get("request"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// Extracts request details from a beforeRequestSent event.
fn parse_pending_request(params: &Map<String, Value>) -> Option<PendingRequest> {
    let req = params.get("request").and_then(Value::as_object)?;
    let id = req.get("request").and_then(Value::as_str).unwrap_or("");
    if id.is_empty() {
        return None;
    }
    Some(PendingRequest {
        request_id: id.to_string(),
        url: req.get("url").and_then(Value::as_str).unwrap_or("").to_string(),
        method: req.get("method").and_then(Value::as_str).unwrap_or("").to_string(),
        headers: req.get("headers").and_then(Value::as_array).cloned().unwrap_or_default(),
        cookies: req.get("cookies").and_then(Value::as_array).cloned().unwrap_or_default(),
        headers_size: to_f64(req.get("headersSize")),
        body_size: to_f64(req.get("bodySize")),
        context: params.get("context").and_then(Value::as_str).unwrap_or("").to_string(),
        timestamp: to_f64(params.get("timestamp")),
    })
}

/// Creates a minimal PendingRequest from a responseCompleted/fetchError event
/// when no matching beforeRequestSent exists.
fn parse_pending_request_from_response(params: &Map<String, Value>) -> Option<PendingRequest> {
    let req = params.get("request").and_then(Value::as_object)?;
    Some(PendingRequest {
        request_id: req.get("request").and_then(Value::as_str).unwrap_or("").to_string(),
        url: req.get("url").and_then(Value::as_str).unwrap_or("").to_string(),
        method: req.get("method").and_then(Value::as_str).unwrap_or("").to_string(),
        headers: req.get("headers").and_then(Value::as_array).cloned().unwrap_or_default(),
        cookies: req.get("cookies").and_then(Value::as_array).cloned().unwrap_or_default(),
        headers_size: to_f64(req.get("headersSize")),
        body_size: to_f64(req.get("bodySize")),
        context: params.get("context").and_then(Value::as_str).unwrap_or("").to_string(),
        timestamp: to_f64(params.get("timestamp")),
    })
}

/// Builds a Playwright resource-snapshot event from a correlated BiDi request and
/// response (or fetchError).
fn bidi_to_har_entry(
    pending: &PendingRequest,
    response_params: &Map<String, Value>,
    is_fetch_error: bool,
    monotonic_base: i64,
) -> RecordEvent {
    let end_timestamp = to_f64(response_params.get("timestamp"));
    let mut time_delta = 0.0;
    if end_timestamp > 0.0 && pending.timestamp > 0.0 {
        time_delta = end_timestamp - pending.timestamp;
    }

    let mut start_time = pending.timestamp;
    if start_time == 0.0 {
        start_time = now_unix_millis() as f64;
    }

    // Build HAR request
    let har_request = json!({
        "method": pending.method,
        "url": pending.url,
        "httpVersion": "HTTP/1.1",
        "cookies": flatten_bidi_cookies(&pending.cookies),
        "headers": flatten_bidi_headers(&pending.headers),
        "queryString": parse_query_string(&pending.url),
        "headersSize": pending.headers_size,
        "bodySize": pending.body_size,
    });

    // Build HAR response
    let har_response = build_har_response(response_params, is_fetch_error);

    // Context for _frameref
    let mut context = pending.context.clone();
    if let Some(c) = response_params.get("context").and_then(Value::as_str) {
        if !c.is_empty() {
            context = c.to_string();
        }
    }

    // Build startedDateTime as ISO 8601
    let started_date_time = format_rfc3339_nano_utc(start_time as i64);

    let mut entry = obj(json!({
        "startedDateTime": started_date_time,
        "time": time_delta,
        "request": har_request,
        "response": har_response,
        "cache": {},
        "timings": {
            "send": -1,
            "wait": time_delta,
            "receive": -1,
        },
        "_monotonicTime": start_time - monotonic_base as f64,
    }));
    if !context.is_empty() {
        entry.insert("_frameref".to_string(), Value::from(format_page_id(&context)));
    }

    obj(json!({
        "type": "resource-snapshot",
        "snapshot": Value::Object(entry),
    }))
}

/// Creates the HAR response object from BiDi responseCompleted or fetchError params.
fn build_har_response(params: &Map<String, Value>, is_fetch_error: bool) -> Value {
    if is_fetch_error {
        let error_text = params.get("errorText").and_then(Value::as_str).unwrap_or("");
        return json!({
            "status": 0,
            "statusText": "",
            "httpVersion": "HTTP/1.1",
            "cookies": [],
            "headers": [],
            "content": {
                "size": 0.0,
                "mimeType": "",
            },
            "redirectURL": "",
            "headersSize": -1.0,
            "bodySize": 0.0,
            "_failureText": error_text,
        });
    }

    let empty = Map::new();
    let resp = params.get("response").and_then(Value::as_object).unwrap_or(&empty);

    let status = to_f64(resp.get("status"));
    let status_text = resp.get("statusText").and_then(Value::as_str).unwrap_or("");
    let protocol = resp.get("protocol").and_then(Value::as_str).unwrap_or("");
    let mime_type = resp.get("mimeType").and_then(Value::as_str).unwrap_or("");
    let bytes_received = to_f64(resp.get("bytesReceived"));
    let headers = resp.get("headers").and_then(Value::as_array).cloned().unwrap_or_default();

    let http_version = protocol_to_http_version(protocol);

    json!({
        "status": status,
        "statusText": status_text,
        "httpVersion": http_version,
        "cookies": [],
        "headers": flatten_bidi_headers(&headers),
        "content": {
            "size": bytes_received,
            "mimeType": mime_type,
        },
        "redirectURL": "",
        "headersSize": -1.0,
        "bodySize": bytes_received,
    })
}

/// Converts BiDi header format [{name, value: {type, value}}] to HAR format
/// [{name, value}].
fn flatten_bidi_headers(headers: &[Value]) -> Value {
    let mut result = Vec::with_capacity(headers.len());
    for h in headers {
        let hdr = match h.as_object() {
            Some(m) => m,
            None => continue,
        };
        let name = hdr.get("name").and_then(Value::as_str).unwrap_or("");
        let value = match hdr.get("value") {
            Some(Value::Object(v)) => v.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        result.push(json!({ "name": name, "value": value }));
    }
    Value::Array(result)
}

/// Converts BiDi cookies to a simple array (already fairly flat; just ensures
/// the result is non-null).
fn flatten_bidi_cookies(cookies: &[Value]) -> Value {
    Value::Array(cookies.to_vec())
}

/// Extracts query parameters from a URL as HAR queryString entries.
fn parse_query_string(raw_url: &str) -> Value {
    // Mirror net/url: the fragment ('#') is split off first, so a '?' that appears
    // after a '#' is part of the fragment, not the query.
    let before_fragment = match raw_url.split_once('#') {
        Some((before, _)) => before,
        None => raw_url,
    };
    let raw_query = match before_fragment.split_once('?') {
        Some((_, q)) => q,
        None => return Value::Array(Vec::new()),
    };
    if raw_query.is_empty() {
        return Value::Array(Vec::new());
    }
    let mut result = Vec::new();
    for pair in raw_query.split('&') {
        let (name, value) = match pair.split_once('=') {
            Some((n, v)) => (n, v),
            None => (pair, ""),
        };
        let name = query_unescape(name);
        let value = query_unescape(value);
        result.push(json!({ "name": name, "value": value }));
    }
    Value::Array(result)
}

/// Decodes an application/x-www-form-urlencoded component, like Go's
/// url.QueryUnescape ('+' -> space, %XX -> byte). On malformed input, returns
/// the original (Go keeps the undecoded text on error).
fn query_unescape(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                if i + 2 < bytes.len() {
                    let h = hex_val(bytes[i + 1]);
                    let l = hex_val(bytes[i + 2]);
                    match (h, l) {
                        (Some(h), Some(l)) => {
                            out.push((h << 4) | l);
                            i += 3;
                        }
                        _ => return s.to_string(),
                    }
                } else {
                    return s.to_string();
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Maps a BiDi protocol string to an HTTP version string.
fn protocol_to_http_version(protocol: &str) -> &'static str {
    match protocol {
        "h2" | "h2c" => "h2",
        "h3" => "h3",
        "http/1.0" => "HTTP/1.0",
        "http/1.1" | "" => "HTTP/1.1",
        _ => "HTTP/1.1",
    }
}

/// Converts a numeric Value to f64 (mirrors Go's toFloat64).
fn to_f64(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Converts a raw browsing context ID to Playwright-compatible page ID format:
/// "page@" prefix + lowercase hex.
pub fn format_page_id(context_id: &str) -> String {
    format!("page@{}", context_id.to_lowercase())
}

/// Marshals a record event with keys in Playwright-compatible order. "version"
/// (for context-options) and "type" come first, then known fields in priority
/// order, then any remaining keys alphabetically.
fn marshal_event(event: &RecordEvent) -> String {
    const ORDER: &[&str] = &[
        // context-options (version before type to match Playwright)
        "version",
        // Common
        "type",
        // context-options (continued)
        "origin",
        "libraryName",
        "libraryVersion",
        "browserName",
        "platform",
        "wallTime",
        "monotonicTime",
        "sdkLanguage",
        "title",
        "contextId",
        "options",
        // before/after
        "callId",
        "startTime",
        "endTime",
        "class",
        "method",
        "pageId",
        "parentId",
        "params",
        "beforeSnapshot",
        "afterSnapshot",
        "inputSnapshot",
        // screencast-frame
        "sha1",
        "width",
        "height",
        "timestamp",
        "frameSwapWallTime",
        // input
        "point",
        "box",
        // frame-snapshot
        "snapshot",
        // event
        "time",
    ];

    let mut buf = String::from("{");
    let mut first = true;
    let mut written: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for &key in ORDER {
        if let Some(val) = event.get(key) {
            if !first {
                buf.push(',');
            }
            buf.push_str(&serde_json::to_string(key).unwrap_or_default());
            buf.push(':');
            write_go_value(val, &mut buf);
            first = false;
            written.insert(key);
        }
    }

    // Remaining keys alphabetically
    let mut remaining: Vec<&String> = event.keys().filter(|k| !written.contains(k.as_str())).collect();
    remaining.sort();
    for key in remaining {
        if !first {
            buf.push(',');
        }
        buf.push_str(&serde_json::to_string(key).unwrap_or_default());
        buf.push(':');
        write_go_value(&event[key], &mut buf);
        first = false;
    }

    buf.push('}');
    buf
}

/// Serializes a JSON value the way Go's `encoding/json` would, so the trace
/// matches the Go binary byte-for-byte. The only place serde_json diverges from
/// Go here is numbers: Go renders whole-valued `float64` as bare integers
/// (`json.Marshal(float64(200))` → `200`), whereas serde renders `200.0`. Object
/// keys are already sorted (serde_json Map is a BTreeMap without `preserve_order`,
/// matching Go's map-key sorting), so recursion preserves Go's ordering.
fn write_go_value(v: &Value, buf: &mut String) {
    match v {
        Value::Null => buf.push_str("null"),
        Value::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => buf.push_str(&format_go_number(n)),
        Value::String(_) => buf.push_str(&serde_json::to_string(v).unwrap_or_default()),
        Value::Array(arr) => {
            buf.push('[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                write_go_value(item, buf);
            }
            buf.push(']');
        }
        Value::Object(map) => {
            buf.push('{');
            let mut first = true;
            for (k, val) in map {
                if !first {
                    buf.push(',');
                }
                buf.push_str(&serde_json::to_string(k).unwrap_or_default());
                buf.push(':');
                write_go_value(val, buf);
                first = false;
            }
            buf.push('}');
        }
    }
}

/// Formats a JSON number like Go's `json.Marshal(float64)`: whole values render
/// as bare integers (`200`, `-1`); fractional values use the shortest round-trip
/// form (which serde and Go's encoder agree on for the ranges seen here).
fn format_go_number(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    if let Some(f) = n.as_f64() {
        // Whole-valued float within safe integer range → bare integer (Go-style).
        if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.007_199_254_740_992e15 {
            return (f as i64).to_string();
        }
    }
    n.to_string()
}

/// Decodes a standard base64 string.
pub fn decode_base64(s: &str) -> anyhow::Result<Vec<u8>> {
    Ok(STANDARD.decode(s.as_bytes())?)
}

/// Reads width and height from a PNG file's IHDR chunk. Returns (0, 0) if not a
/// valid PNG.
fn png_dimensions(data: &[u8]) -> (i64, i64) {
    // 8-byte signature + 4-byte chunk length + 4-byte "IHDR" + 4-byte width + 4-byte height
    if data.len() < 24 {
        return (0, 0);
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as i64;
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]) as i64;
    (w, h)
}

/// Reads width and height from a JPEG file's SOF0 marker. Returns (0, 0) if not a
/// valid JPEG.
fn jpeg_dimensions(data: &[u8]) -> (i64, i64) {
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return (0, 0); // not a JPEG
    }
    let mut i = 2usize;
    while i + 1 < data.len() {
        if data[i] != 0xFF {
            return (0, 0);
        }
        let marker = data[i + 1];
        i += 2;
        // Skip padding bytes (0xFF fill)
        if marker == 0xFF {
            i -= 1;
            continue;
        }
        // SOI, RST0-RST7 and TEM have no payload
        if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            continue;
        }
        // EOI or SOS — stop scanning
        if marker == 0xD9 || marker == 0xDA {
            return (0, 0);
        }
        // Read segment length
        if i + 2 > data.len() {
            return (0, 0);
        }
        let seg_len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        // SOF0 (0xC0) through SOF3 (0xC3) contain dimensions
        if (0xC0..=0xC3).contains(&marker) {
            if i + seg_len > data.len() || seg_len < 7 {
                return (0, 0);
            }
            // Offset within segment: 2 (length) + 1 (precision) + 2 (height) + 2 (width)
            let h = u16::from_be_bytes([data[i + 3], data[i + 4]]) as i64;
            let w = u16::from_be_bytes([data[i + 5], data[i + 6]]) as i64;
            return (w, h);
        }
        i += seg_len;
    }
    (0, 0)
}

/// Detects the image format (PNG or JPEG) and returns (width, height).
pub fn image_dimensions(data: &[u8]) -> (i64, i64) {
    if data.len() >= 8 && data[0] == 0x89 && data[1] == b'P' && data[2] == b'N' && data[3] == b'G' {
        return png_dimensions(data);
    }
    jpeg_dimensions(data)
}

/// Writes recording zip data to a file, creating directories as needed.
pub fn write_record_to_file(data: &[u8], path: &str) -> anyhow::Result<()> {
    let dir = std::path::Path::new(path).parent();
    if let Some(dir) = dir {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)
                .map_err(|e| anyhow::anyhow!("failed to create recording dir: {e}"))?;
        }
    }
    std::fs::write(path, data)?;
    Ok(())
}

/// Returns the unix epoch time in milliseconds.
pub(crate) fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Returns Go's `runtime.GOOS` value for the build target (e.g. "darwin").
fn go_goos() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

/// Formats a unix-millis timestamp as RFC 3339 with nanosecond precision in UTC,
/// matching Go's `time.UnixMilli(ms).UTC().Format(time.RFC3339Nano)` (trailing
/// zeros in the fractional second are trimmed; "Z" denotes UTC).
fn format_rfc3339_nano_utc(unix_millis: i64) -> String {
    let secs = unix_millis.div_euclid(1000);
    let millis = unix_millis.rem_euclid(1000);

    let days = secs.div_euclid(86400);
    let secs_of_day = secs.rem_euclid(86400);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    let (year, month, day) = civil_from_days(days);

    // RFC3339Nano trims trailing zeros from the fractional part (ms precision).
    let frac = if millis == 0 {
        String::new()
    } else {
        let mut f = format!(".{millis:03}");
        while f.ends_with('0') {
            f.pop();
        }
        f
    };

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{frac}Z")
}

/// Converts a count of days since the unix epoch to (year, month, day) in the
/// proleptic Gregorian calendar (Howard Hinnant's civil_from_days).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as u32, d as u32)
}

/// Coerces a JSON Value into an object map (empty if not an object).
fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}
