//! Phase 3 emulation cluster: viewport/window/media/geolocation/setContent routes
//! plus exported standalone helpers used by the MCP agent.

use std::sync::Arc;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::helpers::{check_bidi_error, eval_simple_script};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::{new_api_session, Session};

const EMULATE_MEDIA_SCRIPT: &str = "(overridesJSON) => {\n\
const overrides = JSON.parse(overridesJSON);\n\
if (!window.__browserlaneMediaOverrides) { window.__browserlaneMediaOverrides = {}; }\n\
const featureMap = {\n\
  colorScheme: 'prefers-color-scheme',\n\
  reducedMotion: 'prefers-reduced-motion',\n\
  forcedColors: 'forced-colors',\n\
  contrast: 'prefers-contrast'\n\
};\n\
for (const [key, value] of Object.entries(overrides)) {\n\
  if (value === null) { delete window.__browserlaneMediaOverrides[key]; }\n\
  else { window.__browserlaneMediaOverrides[key] = value; }\n\
}\n\
if (!window.__browserlaneOriginalMatchMedia) {\n\
  window.__browserlaneOriginalMatchMedia = window.matchMedia.bind(window);\n\
  window.matchMedia = function(query) {\n\
    const original = window.__browserlaneOriginalMatchMedia(query);\n\
    const ov = window.__browserlaneMediaOverrides || {};\n\
    if (ov.media !== undefined) {\n\
      const q = query.trim().toLowerCase();\n\
      if (q === 'print' || q === '(print)') return makeResult(original, ov.media === 'print', query);\n\
      if (q === 'screen' || q === '(screen)') return makeResult(original, ov.media === 'screen', query);\n\
    }\n\
    for (const [key, feature] of Object.entries(featureMap)) {\n\
      if (ov[key] !== undefined) {\n\
        const re = new RegExp('\\\\(' + feature + '\\\\s*:\\\\s*([^)]+)\\\\)');\n\
        const m = query.match(re);\n\
        if (m) { return makeResult(original, m[1].trim() === ov[key], query); }\n\
      }\n\
    }\n\
    return original;\n\
  };\n\
}\n\
function makeResult(original, matches, media) {\n\
  return {\n\
    matches: matches, media: media, onchange: original.onchange,\n\
    addListener: original.addListener.bind(original),\n\
    removeListener: original.removeListener.bind(original),\n\
    addEventListener: original.addEventListener.bind(original),\n\
    removeEventListener: original.removeEventListener.bind(original),\n\
    dispatchEvent: original.dispatchEvent.bind(original)\n\
  };\n\
}\n\
return 'ok';\n\
}";

const GEOLOCATION_SCRIPT: &str = "(coordsJSON) => {\n\
const coords = JSON.parse(coordsJSON);\n\
const geo = navigator.geolocation;\n\
geo.getCurrentPosition = function(success, error, options) {\n\
  success({ coords: { latitude: coords.latitude, longitude: coords.longitude, accuracy: coords.accuracy,\n\
    altitude: null, altitudeAccuracy: null, heading: null, speed: null }, timestamp: Date.now() });\n\
};\n\
geo.watchPosition = function(success, error, options) {\n\
  success({ coords: { latitude: coords.latitude, longitude: coords.longitude, accuracy: coords.accuracy,\n\
    altitude: null, altitudeAccuracy: null, heading: null, speed: null }, timestamp: Date.now() });\n\
  return 0;\n\
};\n\
return 'ok';\n\
}";

impl Router {
    /// Handles `browserlane:page.setViewport` — sets the viewport size.
    pub(crate) async fn handle_page_set_viewport(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let width = cmd.params.get("width").and_then(Value::as_f64).unwrap_or(0.0);
        let height = cmd.params.get("height").and_then(Value::as_f64).unwrap_or(0.0);
        if width == 0.0 || height == 0.0 {
            return self.send_error(session, cmd.id, &anyhow!("width and height are required"));
        }

        let mut params = json!({
            "context": context,
            "viewport": {
                "width": width as i64,
                "height": height as i64,
            },
        });
        if let Some(dpr) = cmd.params.get("devicePixelRatio").and_then(Value::as_f64) {
            if dpr > 0.0 {
                params["devicePixelRatio"] = json!(dpr);
            }
        }

        let resp = match self
            .send_internal_command(session, "browsingContext.setViewport", params)
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.viewport` — returns the current viewport size.
    pub(crate) async fn handle_page_viewport(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let val = match eval_simple_script(
            &s,
            &context,
            "() => JSON.stringify({ width: window.innerWidth, height: window.innerHeight })",
        )
        .await
        {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        #[derive(Deserialize)]
        struct ViewportSize {
            width: i64,
            height: i64,
        }
        let size: ViewportSize = match serde_json::from_str(&val) {
            Ok(s) => s,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("failed to parse viewport: {e}")),
        };

        self.send_success(
            session,
            cmd.id,
            json!({ "width": size.width, "height": size.height }),
        );
    }

    /// Handles `browserlane:page.emulateMedia` — overrides CSS media features.
    pub(crate) async fn handle_page_emulate_media(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let mut overrides: Map<String, Value> = Map::new();
        for key in ["media", "colorScheme", "reducedMotion", "forcedColors", "contrast"] {
            if let Some(val) = cmd.params.get(key) {
                if val.is_null() {
                    overrides.insert(key.to_string(), Value::Null);
                } else if let Some(s) = val.as_str() {
                    overrides.insert(key.to_string(), Value::String(s.to_string()));
                }
            }
        }

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = emulate_media(&s, &context, overrides).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.setContent` — replaces the page HTML.
    pub(crate) async fn handle_page_set_content(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let html = cmd.params.get("html").and_then(Value::as_str).unwrap_or("");
        let params = json!({
            "functionDeclaration": "(html) => { document.open(); document.write(html); document.close(); }",
            "target": { "context": context },
            "arguments": [{ "type": "string", "value": html }],
            "awaitPromise": true,
            "resultOwnership": "root",
        });

        let resp = match self.send_internal_command(session, "script.callFunction", params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.setWindow` — sets the OS browser window size, position, or state.
    pub(crate) async fn handle_page_set_window(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let state = cmd.params.get("state").and_then(Value::as_str).unwrap_or("");
        let width = cmd.params.get("width").and_then(Value::as_f64);
        let height = cmd.params.get("height").and_then(Value::as_f64);
        let x = cmd.params.get("x").and_then(Value::as_f64);
        let y = cmd.params.get("y").and_then(Value::as_f64);

        let mut opts = SetWindowOpts {
            state: state.to_string(),
            ..Default::default()
        };
        if let Some(w) = width {
            let w = w as i64;
            opts.width = Some(w);
        }
        if let Some(h) = height {
            let h = h as i64;
            opts.height = Some(h);
        }
        if let Some(xv) = x {
            let xv = xv as i64;
            opts.x = Some(xv);
        }
        if let Some(yv) = y {
            let yv = yv as i64;
            opts.y = Some(yv);
        }

        let lr = match session.launch_result.as_ref() {
            Some(lr) => lr,
            None => {
                return self.send_error(
                    session,
                    cmd.id,
                    &anyhow!("not supported for remote browsers"),
                )
            }
        };

        if let Err(e) = set_window(lr.port, &lr.session_id, opts).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:page.window` — returns the current OS window state and dimensions.
    pub(crate) async fn handle_page_window(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let s = new_api_session(Arc::clone(self), Arc::clone(session), "");
        let win = match get_window(&s).await {
            Ok(w) => w,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.send_success(
            session,
            cmd.id,
            json!({
                "state": win.state,
                "x": win.x,
                "y": win.y,
                "width": win.width,
                "height": win.height,
            }),
        );
    }

    /// Handles `browserlane:page.setGeolocation` — overrides geolocation.
    pub(crate) async fn handle_page_set_geolocation(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let lat = cmd.params.get("latitude").and_then(Value::as_f64);
        let lng = cmd.params.get("longitude").and_then(Value::as_f64);
        if lat.is_none() || lng.is_none() {
            return self.send_error(session, cmd.id, &anyhow!("latitude and longitude are required"));
        }
        let lat = lat.unwrap();
        let lng = lng.unwrap();

        let mut accuracy = 1.0;
        if let Some(acc) = cmd.params.get("accuracy").and_then(Value::as_f64) {
            accuracy = acc;
        }

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = set_geolocation(&s, &context, lat, lng, accuracy).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({}));
    }
}

// ---------------------------------------------------------------------------
// Exported standalone functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Sends a POST request to a chromedriver classic WebDriver endpoint.
pub async fn chromedriver_post(url: &str, body: &Map<String, Value>) -> anyhow::Result<()> {
    let data = serde_json::to_vec(body).map_err(|e| anyhow!("failed to marshal request: {e}"))?;
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(data)
        .send()
        .await
        .map_err(|e| anyhow!("chromedriver request failed: {e}"))?;

    let status = resp.status();
    let resp_body = resp.text().await.unwrap_or_default();
    if status != reqwest::StatusCode::OK {
        return Err(anyhow!(
            "chromedriver error (status {}): {}",
            status.as_u16(),
            resp_body
        ));
    }
    Ok(())
}

/// OS browser window state and dimensions.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub x: i64,
    #[serde(default)]
    pub y: i64,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
}

/// Returns the current OS browser window state and dimensions.
pub async fn get_window(s: &dyn Session) -> anyhow::Result<WindowInfo> {
    let resp = s
        .send_bidi_command("browser.getClientWindows", json!({}))
        .await
        .map_err(|e| anyhow!("failed to get window: {e}"))?;
    check_bidi_error(&resp)?;

    #[derive(Deserialize)]
    struct GetResult {
        #[serde(default, rename = "clientWindows")]
        client_windows: Vec<WindowInfo>,
    }
    #[derive(Deserialize)]
    struct Outer {
        result: GetResult,
    }

    let outer: Outer = serde_json::from_value(resp)
        .map_err(|e| anyhow!("failed to parse getClientWindows: {e}"))?;
    outer
        .result
        .client_windows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no client windows available"))
}

/// Desired window state and/or dimensions.
#[derive(Debug, Default, Clone)]
pub struct SetWindowOpts {
    pub state: String,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

/// Sets the OS browser window size, position, or state via chromedriver HTTP API.
pub async fn set_window(port: u16, session_id: &str, opts: SetWindowOpts) -> anyhow::Result<()> {
    let base_url = format!("http://localhost:{port}/session/{session_id}/window");

    if !opts.state.is_empty() && opts.state != "normal" {
        let endpoint = match opts.state.as_str() {
            "maximized" => format!("{base_url}/maximize"),
            "minimized" => format!("{base_url}/minimize"),
            "fullscreen" => format!("{base_url}/fullscreen"),
            other => return Err(anyhow!("unsupported window state: {other}")),
        };
        return chromedriver_post(&endpoint, &Map::new()).await;
    }

    let mut rect = Map::new();
    if let Some(w) = opts.width {
        rect.insert("width".to_string(), json!(w));
    }
    if let Some(h) = opts.height {
        rect.insert("height".to_string(), json!(h));
    }
    if let Some(x) = opts.x {
        rect.insert("x".to_string(), json!(x));
    }
    if let Some(y) = opts.y {
        rect.insert("y".to_string(), json!(y));
    }
    chromedriver_post(&format!("{base_url}/rect"), &rect).await
}

/// Overrides CSS media features in the browser via a JS matchMedia override.
pub async fn emulate_media(
    s: &dyn Session,
    context: &str,
    overrides: Map<String, Value>,
) -> anyhow::Result<()> {
    let overrides_json = serde_json::to_string(&overrides)
        .map_err(|e| anyhow!("failed to serialize overrides: {e}"))?;

    let params = json!({
        "functionDeclaration": EMULATE_MEDIA_SCRIPT,
        "target": { "context": context },
        "arguments": [{ "type": "string", "value": overrides_json }],
        "awaitPromise": false,
        "resultOwnership": "root",
    });

    let resp = s.send_bidi_command("script.callFunction", params).await?;
    check_bidi_error(&resp)
}

/// Overrides the browser geolocation via a JS override.
pub async fn set_geolocation(
    s: &dyn Session,
    context: &str,
    lat: f64,
    lon: f64,
    accuracy: f64,
) -> anyhow::Result<()> {
    let coords_json = serde_json::to_string(&json!({
        "latitude": lat,
        "longitude": lon,
        "accuracy": accuracy,
    }))
    .unwrap_or_default();

    let params = json!({
        "functionDeclaration": GEOLOCATION_SCRIPT,
        "target": { "context": context },
        "arguments": [{ "type": "string", "value": coords_json }],
        "awaitPromise": false,
        "resultOwnership": "root",
    });

    let resp = s.send_bidi_command("script.callFunction", params).await?;
    check_bidi_error(&resp)
}
