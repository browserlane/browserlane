//! Phase 2 ports the dispatch framework plus the `browser_start` and
//! `browser_navigate` tools (the navigate vertical slice). All other tools hit
//! the faithful default ("unknown tool: <name>"); their bodies are Phase 3/4.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::anyhow;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::http::HeaderMap;

use super::server::{Content, ToolsCallResult};
use crate::api::{self, Session};
use crate::bidi;
use crate::browser::{self, LaunchOptions, LaunchResult};

/// Manages browser session state and executes tool calls.
pub struct Handlers {
    launch_result: Option<LaunchResult>,
    client: Option<Arc<bidi::Client>>,
    conn: Option<Arc<bidi::Connection>>,
    #[allow(dead_code)]
    screenshot_dir: String,
    headless: bool,
    connect_url: String,
    connect_headers: Option<HeaderMap>,
    active_context: String,
    /// @e1 -> CSS selector, populated by find / find_all.
    ref_map: HashMap<String, String>,
    /// Last map output, used by browser_diff_map.
    last_map: String,
    /// Download directory set via browser_download_set_dir (write-only, mirrors Go).
    #[allow(dead_code)]
    download_dir: String,
    /// Active recorder (Go's `h.recorder`); None when not recording.
    recorder: Option<Arc<api::Recorder>>,
    /// Bounding box of the last resolved element, written by AgentSession's
    /// on_box_set callback so Call() can include it in record_action_end.
    last_element_box: Arc<Mutex<Option<api::BoxInfo>>>,
}

/// Creates a new Handlers instance.
pub fn new_handlers(
    screenshot_dir: &str,
    headless: bool,
    connect_url: &str,
    connect_headers: Option<HeaderMap>,
) -> Handlers {
    Handlers {
        launch_result: None,
        client: None,
        conn: None,
        screenshot_dir: screenshot_dir.to_string(),
        headless,
        connect_url: connect_url.to_string(),
        connect_headers,
        active_context: String::new(),
        ref_map: HashMap::new(),
        last_map: String::new(),
        download_dir: String::new(),
        recorder: None,
        last_element_box: Arc::new(Mutex::new(None)),
    }
}

impl Handlers {
    /// Executes a tool by name with the given arguments. When recording is
    /// active, wraps dispatch with record_action/record_action_end to produce
    /// before/after events (matching the API path), and captures a screenshot
    /// after each non-recording action completes.
    pub async fn call(&mut self, name: &str, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        let recording = self.recorder.as_ref().is_some_and(|r| r.is_recording());

        // Inject a synthetic find trace event before selector-based actions so
        // CLI recordings match the JS client's find→action pairs. Skip @e refs —
        // those come from an explicit find the user already ran.
        if recording && needs_find_step(name) {
            if let Some(sel) = args.get("selector").and_then(Value::as_str) {
                if !sel.is_empty() && !sel.starts_with("@e") {
                    self.record_find_step(sel).await;
                }
            }
        }

        let mut call_id = String::new();
        if recording && !is_recording_command(name) {
            let rec = self.recorder.clone().unwrap();
            call_id = rec.next_call_id();
            let page_id = self.get_context().await;
            // Resolve @e1 refs to real selectors so the trace shows meaningful selectors.
            let record_args = self.resolve_refs_in_args(&args);
            rec.record_action(&call_id, mcp_tool_to_method(name), &record_args, "", &page_id);
            *self.last_element_box.lock().unwrap() = None;
        }

        let result = self.dispatch(name, args).await;

        let end_time = api::now_unix_millis();

        // Read and clear the element box stashed by AgentSession's on_box_set.
        let box_ = self.last_element_box.lock().unwrap().take();

        // Per-action screenshot: capture after successful non-recording commands.
        if result.is_ok() && !is_recording_command(name) {
            if let Some(rec) = self.recorder.clone() {
                if rec.is_recording() {
                    let s = self.new_session();
                    api::capture_recording_screenshot(&s, &rec, end_time).await;
                }
            }
        }

        if !call_id.is_empty() {
            if let Some(rec) = self.recorder.clone() {
                rec.record_action_end(&call_id, "", end_time, box_);
            }
        }

        result
    }

    /// Routes a tool call to the appropriate handler.
    async fn dispatch(&mut self, name: &str, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        match name {
            "browser_start" => self.browser_launch(args).await,
            "browser_navigate" => self.browser_navigate(args).await,
            "browser_back" => self.browser_back(args).await,
            "browser_forward" => self.browser_forward(args).await,
            "browser_reload" => self.browser_reload(args).await,
            "browser_find" => self.browser_find(args).await,
            "browser_find_all" => self.browser_find_all(args).await,
            "browser_count" => self.browser_count(args).await,
            "browser_get_text" => self.browser_get_text(args).await,
            "browser_get_html" => self.browser_get_html(args).await,
            "browser_get_value" => self.browser_get_value(args).await,
            "browser_get_attribute" => self.browser_get_attribute(args).await,
            "browser_is_visible" => self.browser_is_visible(args).await,
            "browser_is_enabled" => self.browser_is_enabled(args).await,
            "browser_is_checked" => self.browser_is_checked(args).await,
            "browser_click" => self.browser_click(args).await,
            "browser_dblclick" => self.browser_dblclick(args).await,
            "browser_type" => self.browser_type(args).await,
            "browser_fill" => self.browser_fill(args).await,
            "browser_press" => self.browser_press(args).await,
            "browser_hover" => self.browser_hover(args).await,
            "browser_focus" => self.browser_focus(args).await,
            "browser_select" => self.browser_select(args).await,
            "browser_check" => self.browser_check(args).await,
            "browser_uncheck" => self.browser_uncheck(args).await,
            "browser_scroll_into_view" => self.browser_scroll_into_view(args).await,
            "browser_drag" => self.browser_drag(args).await,
            "browser_new_page" => self.browser_new_page(args).await,
            "browser_list_pages" => self.browser_list_pages(args).await,
            "browser_switch_page" => self.browser_switch_page(args).await,
            "browser_close_page" => self.browser_close_page(args).await,
            "browser_keys" => self.browser_keys(args).await,
            "browser_scroll" => self.browser_scroll(args).await,
            "browser_mouse_move" => self.browser_mouse_move(args).await,
            "browser_mouse_down" => self.browser_mouse_down(args).await,
            "browser_mouse_up" => self.browser_mouse_up(args).await,
            "browser_mouse_click" => self.browser_mouse_click(args).await,
            "browser_evaluate" => self.browser_evaluate(args).await,
            "browser_screenshot" => self.browser_screenshot(args).await,
            "browser_pdf" => self.browser_pdf(args).await,
            "browser_wait" => self.browser_wait(args).await,
            "browser_highlight" => self.browser_highlight(args).await,
            "browser_wait_for_text" => self.browser_wait_for_text(args).await,
            "browser_wait_for_fn" => self.browser_wait_for_fn(args).await,
            "browser_get_url" => self.browser_get_url(args).await,
            "browser_get_title" => self.browser_get_title(args).await,
            "browser_wait_for_url" => self.browser_wait_for_url(args).await,
            "browser_wait_for_load" => self.browser_wait_for_load(args).await,
            "browser_sleep" => self.browser_sleep(args).await,
            "browser_set_viewport" => self.browser_set_viewport(args).await,
            "browser_get_viewport" => self.browser_get_viewport(args).await,
            "browser_set_window" => self.browser_set_window(args).await,
            "browser_get_window" => self.browser_get_window(args).await,
            "browser_emulate_media" => self.browser_emulate_media(args).await,
            "browser_set_geolocation" => self.browser_set_geolocation(args).await,
            "browser_set_content" => self.browser_set_content(args).await,
            "browser_get_cookies" => self.browser_get_cookies(args).await,
            "browser_set_cookie" => self.browser_set_cookie(args).await,
            "browser_delete_cookies" => self.browser_delete_cookies(args).await,
            "browser_storage_state" => self.browser_storage_state(args).await,
            "browser_restore_storage" => self.browser_restore_storage(args).await,
            "browser_dialog_accept" => self.browser_dialog_accept(args).await,
            "browser_dialog_dismiss" => self.browser_dialog_dismiss(args).await,
            "page_clock_install" => self.page_clock_install(args).await,
            "page_clock_fast_forward" => self.page_clock_fast_forward(args).await,
            "page_clock_run_for" => self.page_clock_run_for(args).await,
            "page_clock_pause_at" => self.page_clock_pause_at(args).await,
            "page_clock_resume" => self.page_clock_resume(args).await,
            "page_clock_set_fixed_time" => self.page_clock_set_fixed_time(args).await,
            "page_clock_set_system_time" => self.page_clock_set_system_time(args).await,
            "page_clock_set_timezone" => self.page_clock_set_timezone(args).await,
            "browser_download_set_dir" => self.browser_download_set_dir(args).await,
            "browser_a11y_tree" => self.browser_a11y_tree(args).await,
            "browser_frames" => self.browser_frames(args).await,
            "browser_frame" => self.browser_frame(args).await,
            "browser_upload" => self.browser_upload(args).await,
            "browser_record_start" => self.browser_record_start(args).await,
            "browser_record_stop" => self.browser_record_stop(args).await,
            "browser_record_start_group" => self.browser_record_start_group(args).await,
            "browser_record_stop_group" => self.browser_record_stop_group(args).await,
            "browser_record_start_chunk" => self.browser_record_start_chunk(args).await,
            "browser_record_stop_chunk" => self.browser_record_stop_chunk(args).await,
            "browser_map" => self.browser_map(args).await,
            "browser_diff_map" => self.browser_diff_map(args).await,
            "browser_stop" => self.browser_quit(args).await,
            // ext-seam (browserlane extension hook)
            _ => match crate::ext::dispatch_mcp_tool(name, args).await {
                Some(r) => r,
                None => Err(anyhow!("unknown tool: {name}")),
            },
        }
    }

    /// Launches the browser if not already running.
    async fn ensure_browser(&mut self) -> anyhow::Result<()> {
        if self.client.is_none() {
            self.browser_launch(Map::new())
                .await
                .map_err(|e| anyhow!("auto-launch failed: {e}"))?;
        }
        Ok(())
    }

    /// Launches a new browser session or connects to a remote one.
    async fn browser_launch(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        if self.client.is_some() {
            return Ok(text_result("Browser already running"));
        }

        if !self.connect_url.is_empty() {
            let (conn, client, session_id) =
                bidi::connect_remote(&self.connect_url, self.connect_headers.clone())
                    .await
                    .map_err(|e| anyhow!("failed to connect to remote browser: {e}"))?;
            self.conn = Some(conn);
            self.client = Some(Arc::new(client));
            return Ok(text_result(&format!(
                "Connected to remote browser at {} (session {session_id})",
                self.connect_url
            )));
        }

        let use_headless = args
            .get("headless")
            .and_then(Value::as_bool)
            .unwrap_or(self.headless);

        let lr = browser::launch(LaunchOptions {
            headless: use_headless,
            port: 0,
            verbose: false,
        })
        .await
        .map_err(|e| anyhow!("failed to launch browser: {e}"))?;

        let conn = match lr.bidi_conn.clone() {
            Some(c) => c,
            None => match bidi::connect(&lr.web_socket_url).await {
                Ok(c) => Arc::new(c),
                Err(e) => {
                    let _ = lr.close().await;
                    return Err(anyhow!("failed to connect to browser: {e}"));
                }
            },
        };

        self.client = Some(Arc::new(bidi::Client::new(Arc::clone(&conn))));
        self.conn = Some(conn);
        self.launch_result = Some(lr);

        Ok(text_result(&format!(
            "Browser launched (headless: {use_headless})"
        )))
    }

    /// Navigates to a URL.
    async fn browser_navigate(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let url = args.get("url").and_then(Value::as_str).unwrap_or("");
        if url.is_empty() {
            return Err(anyhow!("url is required"));
        }

        let client = Arc::clone(self.client.as_ref().unwrap());
        let mut s = api::new_agent_session(client);
        s.context = self.active_context.clone();

        let ctx = s.get_context_id().await?;
        api::navigate(&s, &ctx, url, "complete")
            .await
            .map_err(|e| anyhow!("failed to navigate: {e}"))?;

        Ok(text_result(&format!("Navigated to {url}")))
    }

    /// Navigates back in history.
    async fn browser_back(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        api::go_back(&s, &ctx)
            .await
            .map_err(|e| anyhow!("failed to go back: {e}"))?;
        Ok(text_result("Navigated back"))
    }

    /// Navigates forward in history.
    async fn browser_forward(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        api::go_forward(&s, &ctx)
            .await
            .map_err(|e| anyhow!("failed to go forward: {e}"))?;
        Ok(text_result("Navigated forward"))
    }

    /// Reloads the current page.
    async fn browser_reload(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        api::reload(&s, &ctx, "complete")
            .await
            .map_err(|e| anyhow!("failed to reload: {e}"))?;
        Ok(text_result("Page reloaded"))
    }

    /// Finds an element by CSS selector or semantic locator, stores it as @e1.
    async fn browser_find(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let g = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let role = g("role");
        let text = g("text");
        let label = g("label");
        let placeholder = g("placeholder");
        let testid = g("testid");
        let xpath = g("xpath");
        let alt = g("alt");
        let title = g("title");

        let has_semantic = !role.is_empty()
            || !text.is_empty()
            || !label.is_empty()
            || !placeholder.is_empty()
            || !testid.is_empty()
            || !xpath.is_empty()
            || !alt.is_empty()
            || !title.is_empty();

        let client = Arc::clone(self.client.as_ref().unwrap());

        if has_semantic {
            let timeout = match arg_float(&args, "timeout") {
                Some(t) => Duration::from_millis(t as u64),
                None => api::DEFAULT_TIMEOUT,
            };
            let script = find_by_semantic_script();
            let call_args = vec![
                json!(role),
                json!(text),
                json!(label),
                json!(placeholder),
                json!(testid),
                json!(xpath),
                json!(alt),
                json!(title),
            ];
            let result = poll_call_function(&client, &script, call_args, timeout).await;
            let result = match result {
                Ok(r) => r,
                Err(_) => {
                    let mut desc = String::new();
                    for (k, v) in [
                        ("role", &role),
                        ("text", &text),
                        ("label", &label),
                        ("placeholder", &placeholder),
                        ("testid", &testid),
                        ("xpath", &xpath),
                        ("alt", &alt),
                        ("title", &title),
                    ] {
                        if !v.is_empty() {
                            if !desc.is_empty() {
                                desc.push_str(", ");
                            }
                            desc.push_str(&format!("{k}={v}"));
                        }
                    }
                    return Err(anyhow!(
                        "element not found: {desc} (timeout {})",
                        crate::errors::format_go_duration(timeout)
                    ));
                }
            };

            let found: FindResult = serde_json::from_str(result.as_str().unwrap_or(""))
                .map_err(|e| anyhow!("failed to parse find result: {e}"))?;

            self.ref_map.clear();
            self.ref_map.insert("@e1".to_string(), found.selector);

            return Ok(text_result(&format!("@e1 {}", found.label)));
        }

        // CSS selector mode.
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!(
                "selector or semantic locator (role, text, label, placeholder, testid, xpath, alt, title) is required"
            ));
        }
        let selector = self.resolve_selector(selector);

        let label_script = format!(
            "(selector) => {{\n\t\t{gl}\n\t\tconst el = document.querySelector(selector);\n\t\tif (!el) return null;\n\t\tif (el.scrollIntoViewIfNeeded) {{\n\t\t\tel.scrollIntoViewIfNeeded(true);\n\t\t}} else {{\n\t\t\tel.scrollIntoView({{ block: 'center', inline: 'nearest' }});\n\t\t}}\n\t\treturn getLabel(el);\n\t}}",
            gl = get_label_js()
        );
        let label_result = client
            .call_function("", &label_script, vec![json!(selector)])
            .await?;

        self.ref_map.clear();
        self.ref_map.insert("@e1".to_string(), selector);

        let label_str = label_result.as_str().unwrap_or("").to_string();
        Ok(text_result(&format!("@e1 {label_str}")))
    }

    /// Finds all elements matching a CSS selector, storing @e1.. refs.
    async fn browser_find_all(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let limit = arg_float(&args, "limit").map(|l| l as i64).unwrap_or(10);

        let find_all_script = format!(
            "(selector, limit) => {{\n\t\t{gs}\n\t\t{gl}\n\t\tconst els = document.querySelectorAll(selector);\n\t\tconst results = [];\n\t\tconst n = Math.min(els.length, limit);\n\t\tfor (let i = 0; i < n; i++) {{\n\t\t\tconst el = els[i];\n\t\t\tresults.push({{ selector: getSelector(el), label: getLabel(el) }});\n\t\t}}\n\t\treturn JSON.stringify(results);\n\t}}",
            gs = get_selector_js(),
            gl = get_label_js()
        );
        let client = Arc::clone(self.client.as_ref().unwrap());
        let result = client
            .call_function("", &find_all_script, vec![json!(selector), json!(limit)])
            .await
            .map_err(|e| anyhow!("failed to find elements: {e}"))?;

        let elements: Vec<FindResult> = serde_json::from_str(result.as_str().unwrap_or(""))
            .map_err(|e| anyhow!("failed to parse find-all results: {e}"))?;

        self.ref_map.clear();
        let mut lines: Vec<String> = Vec::new();
        for (i, el) in elements.iter().enumerate() {
            let ref_ = format!("@e{}", i + 1);
            self.ref_map.insert(ref_.clone(), el.selector.clone());
            lines.push(format!("{ref_} {}", el.label));
        }

        let text = if lines.is_empty() {
            "No elements found".to_string()
        } else {
            lines.join("\n")
        };
        Ok(text_result(&text))
    }

    /// Counts elements matching a CSS selector.
    async fn browser_count(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        let count = api::get_count(&s, &ctx, &selector)
            .await
            .map_err(|e| anyhow!("failed to count: {e}"))?;

        Ok(text_result(&count.to_string()))
    }

    /// Returns the text content of the page or an element.
    async fn browser_get_text(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;

        let text = match args.get("selector").and_then(Value::as_str) {
            Some(sel) if !sel.is_empty() => {
                let selector = self.resolve_selector(sel);
                api::get_inner_text(&s, &ctx, api::ElementParams { selector, ..Default::default() }).await
            }
            _ => api::eval_simple_script(&s, &ctx, "() => document.body.innerText").await,
        }
        .map_err(|e| anyhow!("failed to get text: {e}"))?;

        Ok(text_result(&text))
    }

    /// Returns the HTML content of the page or an element.
    async fn browser_get_html(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;

        let outer = args.get("outer").and_then(Value::as_bool).unwrap_or(false);

        let html = match args.get("selector").and_then(Value::as_str) {
            Some(sel) if !sel.is_empty() => {
                let selector = self.resolve_selector(sel);
                let ep = api::ElementParams { selector, ..Default::default() };
                if outer {
                    api::get_outer_html(&s, &ctx, ep).await
                } else {
                    api::get_inner_html(&s, &ctx, ep).await
                }
            }
            _ => api::get_content(&s, &ctx).await,
        }
        .map_err(|e| anyhow!("failed to get HTML: {e}"))?;

        Ok(text_result(&html))
    }

    /// Returns the current value of a form element.
    async fn browser_get_value(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        let value = api::get_value(&s, &ctx, api::ElementParams { selector, ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to get value: {e}"))?;

        Ok(text_result(&value))
    }

    /// Gets an HTML attribute value from an element.
    async fn browser_get_attribute(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let attribute = args.get("attribute").and_then(Value::as_str).unwrap_or("");
        if attribute.is_empty() {
            return Err(anyhow!("attribute is required"));
        }

        let (s, ctx) = self.agent_session_with_context().await?;
        let value = api::get_attribute(
            &s,
            &ctx,
            api::ElementParams { selector, ..Default::default() },
            attribute,
        )
        .await
        .map_err(|e| anyhow!("failed to get attribute: {e}"))?;

        Ok(text_result(&value))
    }

    /// Checks if an element is visible on the page.
    async fn browser_is_visible(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        match api::is_visible(&s, &ctx, api::ElementParams { selector, ..Default::default() }).await {
            Ok(visible) => Ok(text_result(&format!("{visible}"))),
            // Element not found or error — return false, not an error.
            Err(_) => Ok(text_result("false")),
        }
    }

    /// Checks if an element is enabled.
    async fn browser_is_enabled(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        let enabled = api::is_enabled(&s, &ctx, api::ElementParams { selector, ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to check enabled: {e}"))?;

        Ok(text_result(&format!("{enabled}")))
    }

    /// Checks if an element is checked.
    async fn browser_is_checked(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        let checked = api::is_checked(&s, &ctx, api::ElementParams { selector, ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to check checked state: {e}"))?;

        Ok(text_result(&format!("{checked}")))
    }

    /// Clicks an element (with actionability checks).
    async fn browser_click(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        let mut ep = api::ElementParams { selector: selector.clone(), ..Default::default() };
        if let Some(t) = arg_float(&args, "timeout") {
            ep.timeout = Duration::from_millis(t as u64);
        }
        api::click(&s, &ctx, ep).await.map_err(|e| anyhow!("failed to click: {e}"))?;

        Ok(text_result(&format!("Clicked element: {selector}")))
    }

    /// Double-clicks an element.
    async fn browser_dblclick(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        api::dbl_click(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to double-click: {e}"))?;

        Ok(text_result(&format!("Double-clicked element: {selector}")))
    }

    /// Types text into an element (clicks to focus first; does not clear).
    async fn browser_type(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let text = match args.get("text").and_then(Value::as_str) {
            Some(t) => t.to_string(),
            None => return Err(anyhow!("text is required")),
        };

        let (s, ctx) = self.agent_session_with_context().await?;
        let mut ep = api::ElementParams { selector: selector.clone(), ..Default::default() };
        if let Some(t) = arg_float(&args, "timeout") {
            ep.timeout = Duration::from_millis(t as u64);
        }
        api::type_into(&s, &ctx, ep, &text).await.map_err(|e| anyhow!("failed to type: {e}"))?;

        Ok(text_result(&format!("Typed into element: {selector}")))
    }

    /// Fills (clears + sets) an input value via JS.
    async fn browser_fill(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let mut value = args.get("value").and_then(Value::as_str).unwrap_or("").to_string();
        if value.is_empty() {
            // Fall back to "text" for backwards compatibility with MCP clients.
            value = args.get("text").and_then(Value::as_str).unwrap_or("").to_string();
        }
        if value.is_empty() {
            return Err(anyhow!("value is required"));
        }

        let (s, ctx) = self.agent_session_with_context().await?;
        api::fill(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() }, &value)
            .await
            .map_err(|e| anyhow!("failed to fill: {e}"))?;

        Ok(text_result(&format!("Filled {value:?} into {selector}")))
    }

    /// Presses a key on a specific element or the focused element.
    async fn browser_press(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let key = match args.get("key").and_then(Value::as_str) {
            Some(k) if !k.is_empty() => k.to_string(),
            _ => return Err(anyhow!("key is required")),
        };

        let (s, ctx) = self.agent_session_with_context().await?;
        // If a selector is given, click to focus first then press the key.
        match args.get("selector").and_then(Value::as_str) {
            Some(sel) if !sel.is_empty() => {
                let selector = self.resolve_selector(sel);
                api::press_on(&s, &ctx, api::ElementParams { selector, ..Default::default() }, &key)
                    .await
                    .map_err(|e| anyhow!("failed to press key: {e}"))?;
            }
            _ => {
                api::press_key(&s, &ctx, &key).await.map_err(|e| anyhow!("failed to press key: {e}"))?;
            }
        }

        Ok(text_result(&format!("Pressed {key}")))
    }

    /// Hovers over an element.
    async fn browser_hover(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        api::hover(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to hover: {e}"))?;

        Ok(text_result(&format!("Hovered over element: {selector}")))
    }

    /// Focuses an element.
    async fn browser_focus(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        api::focus_element(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to focus: {e}"))?;

        Ok(text_result(&format!("Focused element: {selector}")))
    }

    /// Selects an option in a <select> element.
    async fn browser_select(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let value = args.get("value").and_then(Value::as_str).unwrap_or("");
        if value.is_empty() {
            return Err(anyhow!("value is required"));
        }
        let value = value.to_string();

        let (s, ctx) = self.agent_session_with_context().await?;
        api::select_option(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() }, &value)
            .await
            .map_err(|e| anyhow!("failed to select: {e}"))?;

        Ok(text_result(&format!("Selected value {value:?} in {selector}")))
    }

    /// Checks a checkbox or radio button (idempotent).
    async fn browser_check(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        let toggled = api::check(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to check: {e}"))?;

        let msg = if toggled {
            format!("Checked {selector}")
        } else {
            format!("Already checked: {selector}")
        };
        Ok(text_result(&msg))
    }

    /// Unchecks a checkbox (idempotent).
    async fn browser_uncheck(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        let toggled = api::uncheck(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to uncheck: {e}"))?;

        let msg = if toggled {
            format!("Unchecked {selector}")
        } else {
            format!("Already unchecked: {selector}")
        };
        Ok(text_result(&msg))
    }

    /// Scrolls an element into view.
    async fn browser_scroll_into_view(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let (s, ctx) = self.agent_session_with_context().await?;
        api::scroll_into_view(&s, &ctx, api::ElementParams { selector: selector.clone(), ..Default::default() })
            .await
            .map_err(|e| anyhow!("failed to scroll into view: {e}"))?;

        Ok(text_result(&format!("Scrolled {selector} into view")))
    }

    /// Drags from a source element to a target element.
    async fn browser_drag(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let source = args.get("source").and_then(Value::as_str).unwrap_or("");
        if source.is_empty() {
            return Err(anyhow!("source selector is required"));
        }
        let source = self.resolve_selector(source);

        let target = args.get("target").and_then(Value::as_str).unwrap_or("");
        if target.is_empty() {
            return Err(anyhow!("target selector is required"));
        }
        let target = self.resolve_selector(target);

        let (s, ctx) = self.agent_session_with_context().await?;
        api::drag_to(
            &s,
            &ctx,
            api::ElementParams { selector: source.clone(), ..Default::default() },
            api::ElementParams { selector: target.clone(), ..Default::default() },
        )
        .await
        .map_err(|e| anyhow!("failed to drag: {e}"))?;

        Ok(text_result(&format!("Dragged {source:?} to {target:?}")))
    }

    /// Creates a new page (tab), activates it, and tracks it as the active context.
    async fn browser_new_page(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let url = args.get("url").and_then(Value::as_str).unwrap_or("").to_string();

        let (s, _ctx) = self.agent_session_with_context().await?;
        let context_id = api::new_page(&s, &url).await.map_err(|e| anyhow!("failed to create page: {e}"))?;
        // Activate and track the new page so subsequent commands target it.
        api::switch_page(&s, &context_id)
            .await
            .map_err(|e| anyhow!("failed to activate new page: {e}"))?;
        self.active_context = context_id;

        let msg = if url.is_empty() {
            "New page opened".to_string()
        } else {
            format!("New page opened and navigated to {url}")
        };
        Ok(text_result(&msg))
    }

    /// Lists all open browser pages.
    async fn browser_list_pages(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let (s, _ctx) = self.agent_session_with_context().await?;
        let pages = api::list_pages(&s).await.map_err(|e| anyhow!("failed to get pages: {e}"))?;

        let mut text = String::new();
        for (i, page) in pages.iter().enumerate() {
            text.push_str(&format!("[{i}] {}\n", page.url));
        }
        if text.is_empty() {
            text = "No pages open".to_string();
        }
        Ok(text_result(&text))
    }

    /// Switches to a page by index or URL substring.
    async fn browser_switch_page(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let (s, _ctx) = self.agent_session_with_context().await?;
        let pages = api::list_pages(&s).await.map_err(|e| anyhow!("failed to get pages: {e}"))?;

        let context_id = if let Some(idx) = arg_float(&args, "index") {
            let i = idx as i64;
            if i < 0 || i as usize >= pages.len() {
                return Err(anyhow!("page index {i} out of range (0-{})", pages.len() as i64 - 1));
            }
            pages[i as usize].context.clone()
        } else if let Some(url) = args.get("url").and_then(Value::as_str).filter(|u| !u.is_empty()) {
            match pages.iter().find(|p| p.url.contains(url)) {
                Some(p) => p.context.clone(),
                None => return Err(anyhow!("no page matching URL {url:?}")),
            }
        } else {
            return Err(anyhow!("index or url is required"));
        };

        api::switch_page(&s, &context_id).await?;
        self.active_context = context_id.clone();
        Ok(text_result(&format!("Switched to page: {context_id}")))
    }

    /// Closes a page by index (default: the active page).
    async fn browser_close_page(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let (s, _ctx) = self.agent_session_with_context().await?;
        let pages = api::list_pages(&s).await.map_err(|e| anyhow!("failed to get pages: {e}"))?;

        if pages.is_empty() {
            return Err(anyhow!("no pages open"));
        }

        let mut idx: i64 = -1;
        if let Some(i) = arg_float(&args, "index") {
            idx = i as i64;
        } else if !self.active_context.is_empty() {
            // No index given — default to the active page.
            if let Some(pos) = pages.iter().position(|p| p.context == self.active_context) {
                idx = pos as i64;
            }
        }
        if idx < 0 {
            idx = 0; // fall back to first page
        }

        if idx < 0 || idx as usize >= pages.len() {
            return Err(anyhow!("page index {idx} out of range (0-{})", pages.len() as i64 - 1));
        }

        let closed_context = pages[idx as usize].context.clone();
        api::close_page(&s, &closed_context).await?;
        if self.active_context == closed_context {
            self.active_context = String::new();
        }
        Ok(text_result(&format!("Closed page {idx}")))
    }

    /// Presses a key or key combination.
    async fn browser_keys(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let keys = match args.get("keys").and_then(Value::as_str) {
            Some(k) if !k.is_empty() => k.to_string(),
            _ => return Err(anyhow!("keys is required")),
        };
        let (s, ctx) = self.agent_session_with_context().await?;
        api::press_key(&s, &ctx, &keys).await.map_err(|e| anyhow!("failed to press keys: {e}"))?;
        Ok(text_result(&format!("Pressed keys: {keys}")))
    }

    /// Scrolls the page or an element in a direction.
    async fn browser_scroll(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let direction = match args.get("direction").and_then(Value::as_str) {
            Some(d) if !d.is_empty() => d.to_string(),
            _ => "down".to_string(),
        };
        let amount = arg_float(&args, "amount").map(|a| a as i64).unwrap_or(3);

        let (s, ctx) = self.agent_session_with_context().await?;

        // Determine scroll target coordinates.
        let (x, y): (i64, i64) = match args.get("selector").and_then(Value::as_str) {
            Some(sel) if !sel.is_empty() => {
                let selector = self.resolve_selector(sel);
                let info = api::resolve_element(&s, &ctx, api::ElementParams { selector, ..Default::default() }).await?;
                (
                    (info.box_.x + info.box_.width / 2.0) as i64,
                    (info.box_.y + info.box_.height / 2.0) as i64,
                )
            }
            _ => (400, 300), // viewport center fallback
        };

        // Map direction to deltas (120 pixels per scroll "notch").
        let (mut delta_x, mut delta_y) = (0i64, 0i64);
        let pixels = amount * 120;
        match direction.as_str() {
            "down" => delta_y = pixels,
            "up" => delta_y = -pixels,
            "right" => delta_x = pixels,
            "left" => delta_x = -pixels,
            _ => return Err(anyhow!("invalid direction: {direction:?} (use up, down, left, right)")),
        }

        api::scroll_wheel(&s, &ctx, x, y, delta_x, delta_y)
            .await
            .map_err(|e| anyhow!("failed to scroll: {e}"))?;

        Ok(text_result(&format!("Scrolled {direction} by {amount}")))
    }

    /// Moves the mouse to coordinates.
    async fn browser_mouse_move(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let x = match arg_float(&args, "x") {
            Some(v) => v,
            None => return Err(anyhow!("x is required")),
        };
        let y = match arg_float(&args, "y") {
            Some(v) => v,
            None => return Err(anyhow!("y is required")),
        };
        let (s, ctx) = self.agent_session_with_context().await?;
        api::mouse_move(&s, &ctx, x as i64, y as i64)
            .await
            .map_err(|e| anyhow!("failed to move mouse: {e}"))?;
        Ok(text_result(&format!("Mouse moved to ({}, {})", x as i64, y as i64)))
    }

    /// Presses a mouse button.
    async fn browser_mouse_down(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let button = arg_float(&args, "button").map(|b| b as i64).unwrap_or(0);
        let (s, ctx) = self.agent_session_with_context().await?;
        api::mouse_down(&s, &ctx, button)
            .await
            .map_err(|e| anyhow!("failed to press mouse button: {e}"))?;
        Ok(text_result(&format!("Mouse button {button} pressed")))
    }

    /// Releases a mouse button.
    async fn browser_mouse_up(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let button = arg_float(&args, "button").map(|b| b as i64).unwrap_or(0);
        let (s, ctx) = self.agent_session_with_context().await?;
        api::mouse_up(&s, &ctx, button)
            .await
            .map_err(|e| anyhow!("failed to release mouse button: {e}"))?;
        Ok(text_result(&format!("Mouse button {button} released")))
    }

    /// Clicks at coordinates, or at the current position if none given.
    async fn browser_mouse_click(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let button = arg_float(&args, "button").map(|b| b as i64).unwrap_or(0);
        let (s, ctx) = self.agent_session_with_context().await?;

        let x = arg_float(&args, "x");
        let y = arg_float(&args, "y");
        let mut msg;
        if let (Some(x), Some(y)) = (x, y) {
            api::mouse_click(&s, &ctx, x as i64, y as i64, button)
                .await
                .map_err(|e| anyhow!("failed to click: {e}"))?;
            msg = format!("Clicked at ({}, {})", x as i64, y as i64);
        } else {
            // Click at current position (down + up only).
            api::mouse_down(&s, &ctx, button).await.map_err(|e| anyhow!("failed to click: {e}"))?;
            api::mouse_up(&s, &ctx, button).await.map_err(|e| anyhow!("failed to click: {e}"))?;
            msg = "Clicked at current position".to_string();
        }
        if button != 0 {
            msg += &format!(" with button {button}");
        }
        Ok(text_result(&msg))
    }

    /// Evaluates a JavaScript expression and returns the result as text.
    async fn browser_evaluate(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let expression = match args.get("expression").and_then(Value::as_str) {
            Some(e) if !e.is_empty() => e.to_string(),
            _ => return Err(anyhow!("expression is required")),
        };

        let client = Arc::clone(self.client.as_ref().unwrap());
        let result = client.evaluate("", &expression).await.map_err(|e| anyhow!("failed to evaluate: {e}"))?;

        // Format the result as a string, mirroring Go's browserEvaluate:
        //   case string -> as-is; case nil -> "null"; default -> fmt.Sprintf("%v", v).
        // The value is the raw BiDi remote value (objects/arrays keep their
        // {type,value} structure), so composites must use Go's %v formatting
        // (slices as `[a b c]`, maps as `map[k:v]` with keys sorted) — not JSON.
        let result_text = match result {
            Value::String(s) => s,
            Value::Null => "null".to_string(),
            ref other => go_fmt_v(other),
        };
        Ok(text_result(&result_text))
    }

    /// Captures a screenshot. With a filename, saves a PNG into the screenshot
    /// directory (basename only) and returns a text message; otherwise returns
    /// the base64 PNG as an image content block.
    async fn browser_screenshot(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let full_page = args.get("fullPage").and_then(Value::as_bool).unwrap_or(false);
        let annotate = args.get("annotate").and_then(Value::as_bool).unwrap_or(false);

        // If annotate, run map first to get refs, then inject matching labels.
        if annotate {
            self.browser_map(Map::new())
                .await
                .map_err(|e| anyhow!("failed to map for annotation: {e}"))?;

            // Build ordered list of selectors from ref_map (@e1, @e2, ...).
            let mut selectors: Vec<String> = Vec::with_capacity(self.ref_map.len());
            for i in 1..=self.ref_map.len() {
                let ref_ = format!("@e{i}");
                if let Some(sel) = self.ref_map.get(&ref_) {
                    selectors.push(sel.clone());
                }
            }

            // Pass selectors as a JSON string: the BiDi arg serializer has no array
            // case and stringifies a slice to "[a b c]", so the script would receive
            // a string and `document.querySelector("[")` would throw, crashing
            // annotation on every page (issue #156). Marshal to JSON and parse it
            // back in the page.
            let selectors_json = serde_json::to_string(&selectors)
                .map_err(|e| anyhow!("failed to encode annotation selectors: {e}"))?;

            let annotate_script = r#"(selectorsJSON) => {
			const selectors = JSON.parse(selectorsJSON);
			let count = 0;
			for (let i = 0; i < selectors.length; i++) {
				const el = document.querySelector(selectors[i]);
				if (!el) continue;
				const rect = el.getBoundingClientRect();
				if (rect.width === 0 || rect.height === 0) continue;
				const label = document.createElement('div');
				label.className = '__browserlane_annotation';
				label.textContent = i + 1;
				label.style.cssText = 'position:fixed;z-index:2147483647;background:red;color:white;font:bold 11px sans-serif;padding:1px 4px;border-radius:8px;pointer-events:none;line-height:16px;min-width:16px;text-align:center;left:' + (rect.left - 2) + 'px;top:' + (rect.top - 2) + 'px;';
				document.body.appendChild(label);
				count++;
			}
			return JSON.stringify({count: count});
		}"#;
            let client = Arc::clone(self.client.as_ref().unwrap());
            client
                .call_function("", annotate_script, vec![Value::String(selectors_json)])
                .await
                .map_err(|e| anyhow!("failed to annotate: {e}"))?;
        }

        let (s, ctx) = self.agent_session_with_context().await?;
        let base64_data = api::screenshot(&s, &ctx, full_page)
            .await
            .map_err(|e| anyhow!("failed to capture screenshot: {e}"))?;

        // Clean up annotation labels.
        if annotate {
            let cleanup_script = r#"() => {
			document.querySelectorAll('.__browserlane_annotation').forEach(el => el.remove());
			return 'cleaned';
		}"#;
            let client = Arc::clone(self.client.as_ref().unwrap());
            let _ = client.call_function("", cleanup_script, vec![]).await;
        }

        if let Some(filename) = args.get("filename").and_then(Value::as_str).filter(|f| !f.is_empty()) {
            if self.screenshot_dir.is_empty() {
                return Err(anyhow!("screenshot file saving is disabled (use --screenshot-dir to enable)"));
            }
            std::fs::create_dir_all(&self.screenshot_dir)
                .map_err(|e| anyhow!("failed to create screenshot directory: {e}"))?;

            // Use only the basename to prevent path traversal.
            let safe_name = std::path::Path::new(filename)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| filename.to_string());
            let full_path = std::path::Path::new(&self.screenshot_dir).join(&safe_name);

            let png = STANDARD.decode(base64_data.as_bytes()).map_err(|e| anyhow!("failed to decode screenshot: {e}"))?;
            std::fs::write(&full_path, png).map_err(|e| anyhow!("failed to save screenshot: {e}"))?;
            return Ok(text_result(&format!("Screenshot saved to {}", full_path.display())));
        }

        Ok(image_result(&base64_data, "image/png"))
    }

    /// Prints the page to PDF. With a filename, writes the decoded PDF to that
    /// path; otherwise returns the base64 PDF as text.
    async fn browser_pdf(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let (s, ctx) = self.agent_session_with_context().await?;
        let base64_data = api::print_to_pdf(&s, &ctx).await.map_err(|e| anyhow!("failed to print PDF: {e}"))?;

        if let Some(filename) = args.get("filename").and_then(Value::as_str).filter(|f| !f.is_empty()) {
            let pdf = STANDARD.decode(base64_data.as_bytes()).map_err(|e| anyhow!("failed to decode PDF: {e}"))?;
            let n = pdf.len();
            std::fs::write(filename, pdf).map_err(|e| anyhow!("failed to save PDF: {e}"))?;
            return Ok(text_result(&format!("PDF saved to {filename} ({n} bytes)")));
        }

        Ok(text_result(&base64_data))
    }

    /// Waits for an element to reach a state (attached / visible / hidden).
    async fn browser_wait(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let state = match args.get("state").and_then(Value::as_str) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => "attached".to_string(),
        };

        let (s, ctx) = self.agent_session_with_context().await?;
        let mut ep = api::ElementParams { selector: selector.clone(), timeout: api::DEFAULT_TIMEOUT, ..Default::default() };
        if let Some(t) = arg_float(&args, "timeout") {
            ep.timeout = Duration::from_millis(t as u64);
        }

        match state.as_str() {
            "attached" => {
                api::resolve_element(&s, &ctx, ep).await?;
            }
            "visible" => {
                api::wait_for_visible(&s, &ctx, ep).await?;
            }
            "hidden" => {
                api::wait_for_hidden(&s, &ctx, ep).await?;
            }
            _ => return Err(anyhow!("invalid state: {state:?} (use \"attached\", \"visible\", or \"hidden\")")),
        }

        Ok(text_result(&format!("Element {selector:?} reached state: {state}")))
    }

    /// Highlights an element with a visual overlay for 3 seconds.
    async fn browser_highlight(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
        if selector.is_empty() {
            return Err(anyhow!("selector is required"));
        }
        let selector = self.resolve_selector(selector);

        let client = Arc::clone(self.client.as_ref().unwrap());
        let script = r#"(selector) => {
		const el = document.querySelector(selector);
		if (!el) return 'not_found';
		const prev = el.style.cssText;
		el.style.outline = '3px solid red';
		el.style.outlineOffset = '2px';
		el.style.backgroundColor = 'rgba(255,0,0,0.1)';
		setTimeout(() => { el.style.cssText = prev; }, 3000);
		return 'highlighted';
	}"#;
        let result = client
            .call_function("", script, vec![json!(selector)])
            .await
            .map_err(|e| anyhow!("failed to highlight: {e}"))?;

        if result.as_str() == Some("not_found") {
            return Err(anyhow!("element not found: {selector}"));
        }
        Ok(text_result(&format!("Highlighted {selector} (3 seconds)")))
    }

    /// Waits until the given text appears on the page.
    async fn browser_wait_for_text(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let text = match args.get("text").and_then(Value::as_str) {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => return Err(anyhow!("text is required")),
        };
        let timeout = match arg_float(&args, "timeout") {
            Some(t) => Duration::from_millis(t as u64),
            None => api::DEFAULT_TIMEOUT,
        };

        let (s, ctx) = self.agent_session_with_context().await?;
        api::wait_for_text(&s, &ctx, &text, timeout).await?;
        Ok(text_result(&format!("Text {text:?} found on page")))
    }

    /// Waits until a JS expression returns a truthy value.
    async fn browser_wait_for_fn(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let expression = match args.get("expression").and_then(Value::as_str) {
            Some(e) if !e.is_empty() => e.to_string(),
            _ => return Err(anyhow!("expression is required")),
        };
        let timeout = match arg_float(&args, "timeout") {
            Some(t) => Duration::from_millis(t as u64),
            None => api::DEFAULT_TIMEOUT,
        };

        let (s, ctx) = self.agent_session_with_context().await?;
        let result = api::wait_for_function(&s, &ctx, &expression, timeout).await?;
        Ok(text_result(&format!("Expression returned truthy: {result}")))
    }

    /// Returns the current page URL.
    async fn browser_get_url(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let url = api::get_url(&s, &ctx).await.map_err(|e| anyhow!("failed to get URL: {e}"))?;
        Ok(text_result(&url))
    }

    /// Returns the current page title.
    async fn browser_get_title(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let title = api::get_title(&s, &ctx).await.map_err(|e| anyhow!("failed to get title: {e}"))?;
        Ok(text_result(&title))
    }

    /// Waits until the page URL contains a pattern.
    async fn browser_wait_for_url(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let pattern = match args.get("pattern").and_then(Value::as_str) {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => return Err(anyhow!("pattern is required")),
        };
        let timeout = match arg_float(&args, "timeout") {
            Some(t) => Duration::from_millis(t as u64),
            None => api::DEFAULT_TIMEOUT,
        };
        let (s, ctx) = self.agent_session_with_context().await?;
        let url = api::wait_for_url(&s, &ctx, &pattern, timeout).await?;
        Ok(text_result(&format!("URL matches pattern {pattern:?}: {url}")))
    }

    /// Waits until document.readyState is "complete".
    async fn browser_wait_for_load(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let timeout = match arg_float(&args, "timeout") {
            Some(t) => Duration::from_millis(t as u64),
            None => api::DEFAULT_TIMEOUT,
        };
        let (s, ctx) = self.agent_session_with_context().await?;
        api::wait_for_load(&s, &ctx, "complete", timeout).await?;
        Ok(text_result("Page loaded (readyState: complete)"))
    }

    /// Pauses execution for a number of milliseconds (capped at 30s).
    async fn browser_sleep(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        let mut ms = match arg_float(&args, "ms") {
            Some(m) if m > 0.0 => m,
            _ => return Err(anyhow!("ms is required and must be positive")),
        };
        if ms > 30000.0 {
            ms = 30000.0;
        }
        sleep(Duration::from_millis(ms as u64)).await;
        Ok(text_result(&format!("Slept for {ms} ms")))
    }

    /// Sets the viewport size.
    async fn browser_set_viewport(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let width = match arg_float(&args, "width") {
            Some(w) => w,
            None => return Err(anyhow!("width is required")),
        };
        let height = match arg_float(&args, "height") {
            Some(h) => h,
            None => return Err(anyhow!("height is required")),
        };
        let dpr = arg_float(&args, "devicePixelRatio").unwrap_or(0.0);

        let (s, ctx) = self.agent_session_with_context().await?;
        api::set_viewport(&s, &ctx, width as i64, height as i64, dpr)
            .await
            .map_err(|e| anyhow!("failed to set viewport: {e}"))?;

        let mut msg = format!("Viewport set to {}x{}", width as i64, height as i64);
        if dpr > 0.0 {
            msg.push_str(&format!(" (DPR: {dpr:.1})"));
        }
        Ok(text_result(&msg))
    }

    /// Returns the current viewport dimensions.
    async fn browser_get_viewport(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let result = api::eval_simple_script(
            &s,
            &ctx,
            "() => JSON.stringify({width: window.innerWidth, height: window.innerHeight, devicePixelRatio: window.devicePixelRatio})",
        )
        .await
        .map_err(|e| anyhow!("failed to get viewport: {e}"))?;
        Ok(text_result(&result))
    }

    /// Returns the OS browser window state and dimensions.
    async fn browser_get_window(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, _ctx) = self.agent_session_with_context().await?;
        let win = api::get_window(&s).await?;
        let json_bytes = serde_json::to_string(&win)?;
        Ok(text_result(&json_bytes))
    }

    /// Sets the OS browser window size, position, or state.
    async fn browser_set_window(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        if self.launch_result.is_none() {
            return Ok(error_result("Not supported for remote browsers"));
        }

        let state = args.get("state").and_then(Value::as_str).unwrap_or("");
        let width = arg_float(&args, "width");
        let height = arg_float(&args, "height");
        let x = arg_float(&args, "x");
        let y = arg_float(&args, "y");

        let mut opts = api::SetWindowOpts {
            state: state.to_string(),
            ..Default::default()
        };
        if let Some(w) = width {
            opts.width = Some(w as i64);
        }
        if let Some(h) = height {
            opts.height = Some(h as i64);
        }
        if let Some(xv) = x {
            opts.x = Some(xv as i64);
        }
        if let Some(yv) = y {
            opts.y = Some(yv as i64);
        }

        let lr = self.launch_result.as_ref().unwrap();
        api::set_window(lr.port, &lr.session_id, opts).await?;

        let msg = if !state.is_empty() && state != "normal" {
            format!("Window state set to {state}")
        } else if let (Some(w), Some(h)) = (width, height) {
            let mut msg = format!("Window set to {}x{}", w as i64, h as i64);
            if let (Some(xv), Some(yv)) = (x, y) {
                msg.push_str(&format!(" at ({}, {})", xv as i64, yv as i64));
            }
            msg
        } else {
            "Window updated".to_string()
        };
        Ok(text_result(&msg))
    }

    /// Overrides CSS media features.
    async fn browser_emulate_media(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let mut overrides = Map::new();
        for key in ["media", "colorScheme", "reducedMotion", "forcedColors", "contrast"] {
            if let Some(v) = args.get(key).and_then(Value::as_str) {
                if !v.is_empty() {
                    overrides.insert(key.to_string(), Value::String(v.to_string()));
                }
            }
        }
        if overrides.is_empty() {
            return Err(anyhow!("at least one media feature override is required"));
        }

        let (s, ctx) = self.agent_session_with_context().await?;
        api::emulate_media(&s, &ctx, overrides.clone())
            .await
            .map_err(|e| anyhow!("failed to emulate media: {e}"))?;

        let keys: Vec<&str> = overrides.keys().map(String::as_str).collect();
        Ok(text_result(&format!("Media emulation applied: [{}]", keys.join(" "))))
    }

    /// Overrides the browser geolocation.
    async fn browser_set_geolocation(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let latitude = match arg_float(&args, "latitude") {
            Some(v) => v,
            None => return Err(anyhow!("latitude is required")),
        };
        let longitude = match arg_float(&args, "longitude") {
            Some(v) => v,
            None => return Err(anyhow!("longitude is required")),
        };
        let accuracy = arg_float(&args, "accuracy").unwrap_or(1.0);

        let (s, ctx) = self.agent_session_with_context().await?;
        api::set_geolocation(&s, &ctx, latitude, longitude, accuracy)
            .await
            .map_err(|e| anyhow!("failed to set geolocation: {e}"))?;

        Ok(text_result(&format!(
            "Geolocation set to ({latitude:.6}, {longitude:.6})"
        )))
    }

    /// Replaces the page HTML content.
    async fn browser_set_content(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let html = match args.get("html").and_then(Value::as_str) {
            Some(h) if !h.is_empty() => h,
            _ => return Err(anyhow!("html is required")),
        };

        let (s, ctx) = self.agent_session_with_context().await?;
        api::set_content(&s, &ctx, html)
            .await
            .map_err(|e| anyhow!("failed to set content: {e}"))?;

        Ok(text_result(&format!("Page content set ({} chars)", html.len())))
    }

    /// Accepts a dialog (alert, confirm, prompt).
    async fn browser_dialog_accept(
        &mut self,
        args: Map<String, Value>,
    ) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let text = args.get("text").and_then(Value::as_str).unwrap_or("");

        let (s, ctx) = self.agent_session_with_context().await?;
        api::dialog_accept(&s, &ctx, text)
            .await
            .map_err(|e| anyhow!("failed to accept dialog: {e}"))?;

        let msg = if text.is_empty() {
            "Dialog accepted".to_string()
        } else {
            format!("Dialog accepted with text: {text:?}")
        };
        Ok(text_result(&msg))
    }

    /// Dismisses a dialog.
    async fn browser_dialog_dismiss(
        &mut self,
        _args: Map<String, Value>,
    ) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let (s, ctx) = self.agent_session_with_context().await?;
        api::dialog_dismiss(&s, &ctx)
            .await
            .map_err(|e| anyhow!("failed to dismiss dialog: {e}"))?;

        Ok(text_result("Dialog dismissed"))
    }

    /// Installs the fake clock, optionally setting an initial time / timezone.
    async fn page_clock_install(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        api::eval_simple_script(&s, &ctx, api::CLOCK_SCRIPT)
            .await
            .map_err(|e| anyhow!("failed to install clock: {e}"))?;

        if let Some(time_val) = arg_float(&args, "time") {
            let script =
                format!("() => {{ window.__browserlaneClock.setSystemTime({time_val}); return 'ok'; }}");
            api::eval_simple_script(&s, &ctx, &script)
                .await
                .map_err(|e| anyhow!("failed to set initial time: {e}"))?;
        }

        if let Some(tz) = args.get("timezone").and_then(Value::as_str) {
            if !tz.is_empty() {
                api::set_timezone(&s, &ctx, tz)
                    .await
                    .map_err(|e| anyhow!("failed to set timezone: {e}"))?;
            }
        }

        Ok(text_result("Clock installed"))
    }

    /// Fast-forwards the fake clock.
    async fn page_clock_fast_forward(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let ticks = arg_float(&args, "ticks").ok_or_else(|| anyhow!("ticks is required"))?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let script = format!("() => {{ window.__browserlaneClock.fastForward({ticks}); return 'ok'; }}");
        api::eval_simple_script(&s, &ctx, &script)
            .await
            .map_err(|e| anyhow!("clock.fastForward failed: {e}"))?;
        Ok(text_result(&format!("Fast-forwarded {ticks} ms")))
    }

    /// Advances the fake clock, firing all callbacks.
    async fn page_clock_run_for(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let ticks = arg_float(&args, "ticks").ok_or_else(|| anyhow!("ticks is required"))?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let script = format!("() => {{ window.__browserlaneClock.runFor({ticks}); return 'ok'; }}");
        api::eval_simple_script(&s, &ctx, &script)
            .await
            .map_err(|e| anyhow!("clock.runFor failed: {e}"))?;
        Ok(text_result(&format!("Ran for {ticks} ms")))
    }

    /// Pauses the fake clock at a specific time.
    async fn page_clock_pause_at(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let time_val = arg_float(&args, "time").ok_or_else(|| anyhow!("time is required"))?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let script = format!("() => {{ window.__browserlaneClock.pauseAt({time_val}); return 'ok'; }}");
        api::eval_simple_script(&s, &ctx, &script)
            .await
            .map_err(|e| anyhow!("clock.pauseAt failed: {e}"))?;
        Ok(text_result(&format!("Paused at {time_val}")))
    }

    /// Resumes real-time progression.
    async fn page_clock_resume(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        api::eval_simple_script(&s, &ctx, "() => { window.__browserlaneClock.resume(); return 'ok'; }")
            .await
            .map_err(|e| anyhow!("clock.resume failed: {e}"))?;
        Ok(text_result("Clock resumed"))
    }

    /// Freezes Date.now() at a value.
    async fn page_clock_set_fixed_time(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let time_val = arg_float(&args, "time").ok_or_else(|| anyhow!("time is required"))?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let script = format!("() => {{ window.__browserlaneClock.setFixedTime({time_val}); return 'ok'; }}");
        api::eval_simple_script(&s, &ctx, &script)
            .await
            .map_err(|e| anyhow!("clock.setFixedTime failed: {e}"))?;
        Ok(text_result(&format!("Fixed time set to {time_val}")))
    }

    /// Sets Date.now() without triggering timers.
    async fn page_clock_set_system_time(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let time_val = arg_float(&args, "time").ok_or_else(|| anyhow!("time is required"))?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let script = format!("() => {{ window.__browserlaneClock.setSystemTime({time_val}); return 'ok'; }}");
        api::eval_simple_script(&s, &ctx, &script)
            .await
            .map_err(|e| anyhow!("clock.setSystemTime failed: {e}"))?;
        Ok(text_result(&format!("System time set to {time_val}")))
    }

    /// Overrides or resets the browser timezone.
    async fn page_clock_set_timezone(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;

        let tz = args.get("timezone").and_then(Value::as_str).unwrap_or("");
        if tz.is_empty() {
            api::clear_timezone(&s, &ctx)
                .await
                .map_err(|e| anyhow!("failed to clear timezone: {e}"))?;
            return Ok(text_result("Timezone reset to system default"));
        }

        api::set_timezone(&s, &ctx, tz)
            .await
            .map_err(|e| anyhow!("failed to set timezone: {e}"))?;
        Ok(text_result(&format!("Timezone set to {tz}")))
    }

    /// Sets the download directory (creates it, makes it absolute, and tells the
    /// browser to download there).
    async fn browser_download_set_dir(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let dir = match args.get("path").and_then(Value::as_str) {
            Some(d) if !d.is_empty() => d.to_string(),
            _ => return Err(anyhow!("path is required")),
        };

        // Create directory if it doesn't exist.
        std::fs::create_dir_all(&dir).map_err(|e| anyhow!("failed to create download directory: {e}"))?;

        // Make absolute.
        let abs_dir = std::path::absolute(&dir)
            .map_err(|e| anyhow!("failed to resolve path: {e}"))?
            .to_string_lossy()
            .to_string();

        let client = Arc::clone(self.client.as_ref().unwrap());
        let s = api::new_agent_session(client);
        s.send_bidi_command(
            "browser.setDownloadBehavior",
            json!({
                "downloadBehavior": {
                    "type": "allowed",
                    "destinationFolder": abs_dir,
                }
            }),
        )
        .await
        .map_err(|e| anyhow!("failed to set download directory: {e}"))?;

        self.download_dir = abs_dir.clone();

        Ok(text_result(&format!("Download directory set to {abs_dir}")))
    }

    /// Returns the accessibility tree of the current page.
    async fn browser_a11y_tree(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let interesting_only = match args.get("everything").and_then(Value::as_bool) {
            Some(v) => !v,
            None => true,
        };

        let (s, ctx) = self.agent_session_with_context().await?;
        let result = api::a11y_tree(&s, &ctx, interesting_only, "")
            .await
            .map_err(|e| anyhow!("failed to get accessibility tree: {e}"))?;

        Ok(text_result(&result))
    }

    /// Lists all child frames on the page.
    async fn browser_frames(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let frames = api::list_frames(&s, &ctx)
            .await
            .map_err(|e| anyhow!("failed to get frames: {e}"))?;

        if frames.is_empty() {
            return Ok(text_result("No frames found"));
        }
        let frames_json = serde_json::to_string(&frames).unwrap_or_default();
        Ok(text_result(&frames_json))
    }

    /// Finds a frame by name or URL substring.
    async fn browser_frame(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let name_or_url = match args.get("nameOrUrl").and_then(Value::as_str) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => return Err(anyhow!("nameOrUrl is required")),
        };
        let (s, ctx) = self.agent_session_with_context().await?;
        let frame = api::find_frame(&s, &ctx, &name_or_url)
            .await
            .map_err(|e| anyhow!("failed to find frame: {e}"))?;
        match frame {
            Some(f) => Ok(text_result(&serde_json::to_string(&f).unwrap_or_default())),
            None => Err(anyhow!("no frame matching {name_or_url:?}")),
        }
    }

    /// Sets files on an input[type=file] element.
    async fn browser_upload(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let selector = match args.get("selector").and_then(Value::as_str) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Err(anyhow!("selector is required")),
        };
        let selector = self.resolve_selector(&selector);

        let files_raw = args.get("files").ok_or_else(|| anyhow!("files is required"))?;
        let files: Vec<String> = match files_raw.as_array() {
            Some(arr) => arr.iter().filter_map(|f| f.as_str().map(String::from)).collect(),
            None => return Err(anyhow!("files must be an array of strings")),
        };
        if files.is_empty() {
            return Err(anyhow!("at least one file path is required"));
        }

        let (s, ctx) = self.agent_session_with_context().await?;
        let ep = api::ElementParams { selector: selector.clone(), ..Default::default() };
        api::upload(&s, &ctx, ep, files.clone())
            .await
            .map_err(|e| anyhow!("failed to set files: {e}"))?;

        Ok(text_result(&format!("Set {} file(s) on {}", files.len(), selector)))
    }

    /// Maps interactive page elements, assigning @e refs.
    async fn browser_map(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let scope_arg = match args.get("selector").and_then(Value::as_str) {
            Some(s) if !s.is_empty() => json!(s),
            _ => Value::Null,
        };

        let script = map_script();
        let client = Arc::clone(self.client.as_ref().unwrap());
        let result = client
            .call_function("", &script, vec![scope_arg])
            .await
            .map_err(|e| anyhow!("failed to map elements: {e}"))?;

        let elements: Vec<FindResult> = serde_json::from_str(result.as_str().unwrap_or(""))
            .map_err(|e| anyhow!("failed to parse map results: {e}"))?;

        self.ref_map.clear();
        let mut lines: Vec<String> = Vec::new();
        for (i, el) in elements.iter().enumerate() {
            let ref_ = format!("@e{}", i + 1);
            self.ref_map.insert(ref_.clone(), el.selector.clone());
            lines.push(format!("{ref_} {}", el.label));
        }

        let output = if lines.is_empty() {
            "No interactive elements found".to_string()
        } else {
            lines.join("\n")
        };
        self.last_map = output.clone();

        Ok(text_result(&output))
    }

    /// Compares the current page elements vs the last map.
    async fn browser_diff_map(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        if self.last_map.is_empty() {
            return Err(anyhow!("no previous map to diff against — run browser_map first"));
        }

        let prev_map = self.last_map.clone();
        self.browser_map(args).await?;
        let current_map = self.last_map.clone();

        let prev_lines: Vec<&str> = prev_map.split('\n').collect();
        let curr_lines: Vec<&str> = current_map.split('\n').collect();
        let prev_set: HashSet<&str> = prev_lines.iter().copied().collect();
        let curr_set: HashSet<&str> = curr_lines.iter().copied().collect();

        let mut diff: Vec<String> = Vec::new();
        for l in &prev_lines {
            if !curr_set.contains(l) {
                diff.push(format!("- {l}"));
            }
        }
        for l in &curr_lines {
            if !prev_set.contains(l) {
                diff.push(format!("+ {l}"));
            }
        }

        let output = if diff.is_empty() {
            "No changes detected".to_string()
        } else {
            diff.join("\n")
        };

        Ok(text_result(&output))
    }

    /// Returns all cookies for the current context.
    async fn browser_get_cookies(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let (s, ctx) = self.agent_session_with_context().await?;
        let cookies = api::get_cookies(&s, &ctx)
            .await
            .map_err(|e| anyhow!("failed to get cookies: {e}"))?;

        if cookies.is_empty() {
            return Ok(text_result("No cookies"));
        }

        let lines: Vec<String> = cookies
            .iter()
            .map(|c| {
                format!(
                    "{}={} (domain={}, path={})",
                    c.name, c.value, c.domain, c.path
                )
            })
            .collect();
        Ok(text_result(&lines.join("\n")))
    }

    /// Sets a cookie.
    async fn browser_set_cookie(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let name = match args.get("name").and_then(Value::as_str) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => return Err(anyhow!("name is required")),
        };
        let value = match args.get("value").and_then(Value::as_str) {
            Some(v) => v.to_string(),
            None => return Err(anyhow!("value is required")),
        };
        let domain = args.get("domain").and_then(Value::as_str).unwrap_or("");
        let path = args.get("path").and_then(Value::as_str).unwrap_or("");

        let (s, ctx) = self.agent_session_with_context().await?;
        api::set_cookie(&s, &ctx, &name, &value, domain, path)
            .await
            .map_err(|e| anyhow!("failed to set cookie: {e}"))?;

        Ok(text_result(&format!("Cookie set: {name}={value}")))
    }

    /// Deletes cookies (all, or by name).
    async fn browser_delete_cookies(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let name = args.get("name").and_then(Value::as_str).unwrap_or("");

        let (s, ctx) = self.agent_session_with_context().await?;
        api::delete_cookies(&s, &ctx, name)
            .await
            .map_err(|e| anyhow!("failed to delete cookies: {e}"))?;

        let msg = if name.is_empty() {
            "All cookies deleted".to_string()
        } else {
            format!("Cookie {name:?} deleted")
        };
        Ok(text_result(&msg))
    }

    /// Exports cookies, localStorage, and sessionStorage as a JSON state.
    async fn browser_storage_state(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;
        let client = Arc::clone(self.client.as_ref().unwrap());

        let cookies = client
            .get_cookies("")
            .await
            .map_err(|e| anyhow!("failed to get cookies: {e}"))?;

        let script = r#"JSON.stringify({
		origin: location.origin,
		localStorage: (function() {
			var ls = {};
			for (var i = 0; i < localStorage.length; i++) {
				var key = localStorage.key(i);
				ls[key] = localStorage.getItem(key);
			}
			return ls;
		})(),
		sessionStorage: (function() {
			var ss = {};
			for (var i = 0; i < sessionStorage.length; i++) {
				var key = sessionStorage.key(i);
				ss[key] = sessionStorage.getItem(key);
			}
			return ss;
		})()
	})"#;

        let storage_result = client
            .evaluate("", script)
            .await
            .map_err(|e| anyhow!("failed to get storage: {e}"))?;

        let state = json!({
            "cookies": cookies,
            "storage": storage_result,
        });
        let state_json = serde_json::to_string_pretty(&state).unwrap_or_default();
        Ok(text_result(&state_json))
    }

    /// Restores cookies and storage from a JSON state file.
    async fn browser_restore_storage(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        let path = match args.get("path").and_then(Value::as_str) {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => return Err(anyhow!("path is required")),
        };

        let data = std::fs::read_to_string(&path)
            .map_err(|e| anyhow!("failed to read state file: {e}"))?;

        #[derive(Deserialize)]
        struct State {
            #[serde(default)]
            cookies: Vec<bidi::Cookie>,
            #[serde(default)]
            storage: Value,
        }
        let state: State =
            serde_json::from_str(&data).map_err(|e| anyhow!("failed to parse state file: {e}"))?;

        let client = Arc::clone(self.client.as_ref().unwrap());

        let cookie_count = state.cookies.len();
        for cookie in state.cookies {
            let _ = client.set_cookie("", cookie).await;
        }

        if !state.storage.is_null() {
            let storage_json = serde_json::to_string(&state.storage).unwrap_or_default();
            let script = format!(
                "(function() {{\n\t\t\tvar state = {storage_json};\n\t\t\tif (state.localStorage) {{\n\t\t\t\tfor (var key in state.localStorage) {{\n\t\t\t\t\tlocalStorage.setItem(key, state.localStorage[key]);\n\t\t\t\t}}\n\t\t\t}}\n\t\t\tif (state.sessionStorage) {{\n\t\t\t\tfor (var key in state.sessionStorage) {{\n\t\t\t\t\tsessionStorage.setItem(key, state.sessionStorage[key]);\n\t\t\t\t}}\n\t\t\t}}\n\t\t\treturn 'ok';\n\t\t}})()"
            );
            let _ = client.evaluate("", &script).await;
        }

        Ok(text_result(&format!(
            "Storage state restored from {path} ({cookie_count} cookies)"
        )))
    }

    /// Closes the browser session (browser_stop).
    async fn browser_quit(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        if self.client.is_none() {
            return Ok(text_result("No browser session to close"));
        }
        self.close().await;
        Ok(text_result("Browser session closed"))
    }

    /// Resolves @ref selectors to CSS selectors from the refMap.
    fn resolve_selector(&self, selector: &str) -> String {
        if selector.starts_with("@e") {
            if let Some(resolved) = self.ref_map.get(selector) {
                return resolved.clone();
            }
        }
        selector.to_string()
    }

    /// Builds an AgentSession (active context + box callback) without resolving
    /// its context id. Mirrors Go's `newSession`: writes element box info back to
    /// `last_element_box` so Call() can include it in record_action_end.
    fn new_session(&self) -> api::AgentSession {
        let client = Arc::clone(self.client.as_ref().unwrap());
        let mut s = api::new_agent_session(client);
        s.context = self.active_context.clone();
        let box_slot = Arc::clone(&self.last_element_box);
        s.on_box_set = Some(Box::new(move |b| {
            *box_slot.lock().unwrap() = Some(b);
        }));
        s
    }

    /// Builds an AgentSession (with active context) and resolves its context id.
    async fn agent_session_with_context(&self) -> anyhow::Result<(api::AgentSession, String)> {
        let s = self.new_session();
        let ctx = s.get_context_id().await?;
        Ok((s, ctx))
    }

    /// Returns the first browsing context from the browser tree, or "".
    async fn get_context(&self) -> String {
        let client = match &self.client {
            Some(c) => c,
            None => return String::new(),
        };
        match client.get_tree().await {
            Ok(tree) if !tree.contexts.is_empty() => tree.contexts[0].context.clone(),
            _ => String::new(),
        }
    }

    /// Queries the browser for the current viewport size. Returns None on failure.
    async fn query_viewport(&self) -> Option<Value> {
        let context = self.get_context().await;
        if context.is_empty() {
            return None;
        }
        let s = self.new_session();
        let result = api::eval_simple_script(&s, &context, "() => window.innerWidth + ',' + window.innerHeight")
            .await
            .ok()?;
        let (w_str, h_str) = result.split_once(',')?;
        let w: i64 = w_str.parse().ok()?;
        let h: i64 = h_str.parse().ok()?;
        Some(json!({ "width": w, "height": h }))
    }

    /// Returns a copy of `args` with `selector` resolved from an @ref to a CSS
    /// selector, or `args` unchanged if there is no @ref or no mapping.
    fn resolve_refs_in_args(&self, args: &Map<String, Value>) -> Map<String, Value> {
        let sel = match args.get("selector").and_then(Value::as_str) {
            Some(s) if s.starts_with("@e") => s,
            _ => return args.clone(),
        };
        let resolved = self.resolve_selector(sel);
        if resolved == sel {
            return args.clone();
        }
        let mut cp = args.clone();
        cp.insert("selector".to_string(), Value::from(resolved));
        cp
    }

    /// Emits a complete find trace event (before + screenshot + after) so CLI
    /// recordings produce the same find→action pairs as the JS client.
    async fn record_find_step(&self, selector: &str) {
        let rec = match self.recorder.clone() {
            Some(r) if r.is_recording() => r,
            _ => return,
        };

        let s = self.new_session();
        let ctx = match s.get_context_id().await {
            Ok(c) => c,
            Err(_) => return,
        };

        let call_id = rec.next_call_id();
        let page_id = self.get_context().await;
        let mut params = Map::new();
        params.insert("selector".to_string(), Value::from(selector.to_string()));
        rec.record_action(&call_id, "browserlane:page.find", &params, "", &page_id);

        // Use the API path's polling find (handles page transitions, scrolls into view).
        let mut find_params = Map::new();
        find_params.insert("selector".to_string(), Value::from(selector.to_string()));
        let (script, script_args) = api::build_find_script(&find_params, false);
        let info = api::wait_for_element_with_script(&s, &ctx, &script, script_args, api::DEFAULT_TIMEOUT).await;

        let end_time = api::now_unix_millis();

        // Capture screenshot (element is now scrolled into view by the find script).
        api::capture_recording_screenshot(&s, &rec, end_time).await;

        let box_ = info.as_ref().ok().map(|i| i.box_);
        rec.record_action_end(&call_id, "", end_time, box_);
    }

    /// Starts recording (browser_record_start).
    async fn browser_record_start(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        self.ensure_browser().await?;

        if self.recorder.is_some() {
            return Err(anyhow!("already recording — stop it first"));
        }

        let mut opts = api::parse_recording_options(&args);
        if opts.name.is_empty() {
            opts.name = "record".to_string();
        }
        let name = opts.name.clone();
        let screenshots = opts.screenshots;
        let snapshots = opts.snapshots;

        let recorder = Arc::new(api::new_recorder());
        let viewport = self.query_viewport().await;
        recorder.start(opts, viewport);
        self.recorder = Some(Arc::clone(&recorder));

        // Subscribe to events and feed them to the recorder.
        if let Some(client) = &self.client {
            let _ = client
                .send_command(
                    "session.subscribe",
                    json!({
                        "events": [
                            "network.beforeRequestSent",
                            "network.responseCompleted",
                            "network.fetchError",
                            "log.entryAdded",
                            "browsingContext.userPromptOpened",
                            "browsingContext.downloadWillBegin",
                            "browsingContext.load",
                            "browsingContext.fragmentNavigated",
                        ],
                    }),
                )
                .await;
            let rec_for_handler = Arc::clone(&recorder);
            client.set_event_handler(Some(Arc::new(move |msg: String| {
                rec_for_handler.record_bidi_event(&msg);
            })));
        }

        Ok(text_result(&format!(
            "Recording {name:?} started (screenshots: {screenshots}, snapshots: {snapshots})"
        )))
    }

    /// Stops recording and saves to a ZIP file (browser_record_stop).
    async fn browser_record_stop(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        let recorder = match self.recorder.clone() {
            Some(r) => r,
            None => return Err(anyhow!("no recording in progress")),
        };

        // Stop forwarding events to the recorder.
        if let Some(client) = &self.client {
            client.set_event_handler(None);
        }

        // Stop screenshot loop before stopping the recorder.
        recorder.stop_screenshots();

        let path = args
            .get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .unwrap_or("record.zip")
            .to_string();

        let zip_data = match recorder.stop() {
            Ok(d) => d,
            Err(e) => {
                self.recorder = None;
                return Err(anyhow!("failed to stop recording: {e}"));
            }
        };

        if let Err(e) = api::write_record_to_file(&zip_data, &path) {
            self.recorder = None;
            return Err(anyhow!("failed to write recording: {e}"));
        }

        self.recorder = None;

        Ok(text_result(&format!("Recording saved to {path}")))
    }

    /// Starts a named group in the recording (browser_record_start_group).
    async fn browser_record_start_group(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        let recorder = match self.recorder.clone() {
            Some(r) => r,
            None => return Err(anyhow!("no recording in progress")),
        };

        let name = args.get("name").and_then(Value::as_str).unwrap_or("");
        if name.is_empty() {
            return Err(anyhow!("name is required"));
        }

        recorder.start_group(name);

        Ok(text_result(&format!("Started group {name:?}")))
    }

    /// Ends the current group in the recording (browser_record_stop_group).
    async fn browser_record_stop_group(&mut self, _args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        let recorder = match self.recorder.clone() {
            Some(r) => r,
            None => return Err(anyhow!("no recording in progress")),
        };

        recorder.stop_group();

        Ok(text_result("Stopped group"))
    }

    /// Starts a new chunk within the current recording (browser_record_start_chunk).
    async fn browser_record_start_chunk(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        let recorder = match self.recorder.clone() {
            Some(r) => r,
            None => return Err(anyhow!("no recording in progress")),
        };

        let name = args.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let title = args.get("title").and_then(Value::as_str).unwrap_or("").to_string();

        let viewport = self.query_viewport().await;
        recorder.start_chunk(&name, &title, viewport);

        Ok(text_result("Started new recording chunk"))
    }

    /// Packages the current chunk into a zip file (browser_record_stop_chunk).
    /// Recording remains active for additional chunks.
    async fn browser_record_stop_chunk(&mut self, args: Map<String, Value>) -> anyhow::Result<ToolsCallResult> {
        let recorder = match self.recorder.clone() {
            Some(r) => r,
            None => return Err(anyhow!("no recording in progress")),
        };

        let path = args
            .get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .unwrap_or("chunk.zip")
            .to_string();

        let zip_data = recorder
            .stop_chunk()
            .map_err(|e| anyhow!("failed to stop chunk: {e}"))?;

        api::write_record_to_file(&zip_data, &path).map_err(|e| anyhow!("failed to write chunk: {e}"))?;

        Ok(text_result(&format!("Chunk saved to {path}")))
    }

    /// Closes the browser session.
    pub async fn close(&mut self) {
        // Remote mode: end the BiDi session so chromedriver closes Chrome.
        if !self.connect_url.is_empty() {
            if let Some(client) = &self.client {
                let _ = client.send_command("session.end", json!({})).await;
            }
        }
        if let Some(conn) = &self.conn {
            let _ = conn.close().await;
        }
        self.conn = None;
        if let Some(lr) = &self.launch_result {
            let _ = lr.close().await;
        }
        self.launch_result = None;
        self.client = None;
    }
}

/// Returns true for commands that manage recording state (so screenshots aren't
/// captured of recording operations themselves).
fn is_recording_command(name: &str) -> bool {
    matches!(
        name,
        "browser_record_start"
            | "browser_record_stop"
            | "browser_record_start_group"
            | "browser_record_stop_group"
            | "browser_record_start_chunk"
            | "browser_record_stop_chunk"
            | "browser_screenshot"
    )
}

/// Returns true for selector-based action commands (not find itself). These get
/// a synthetic find trace event injected before dispatch so CLI recordings match
/// the JS client's find→action pairs.
fn needs_find_step(name: &str) -> bool {
    matches!(
        name,
        "browser_click"
            | "browser_dblclick"
            | "browser_fill"
            | "browser_type"
            | "browser_press"
            | "browser_hover"
            | "browser_select"
            | "browser_check"
            | "browser_uncheck"
            | "browser_focus"
            | "browser_scroll_into_view"
            | "browser_drag"
            | "browser_get_text"
            | "browser_get_html"
            | "browser_get_value"
            | "browser_get_attribute"
            | "browser_is_visible"
            | "browser_is_enabled"
            | "browser_is_checked"
            | "browser_upload"
            | "browser_highlight"
    )
}

/// Maps an MCP tool name to a browserlane: method name so the trace viewer shows the
/// same action titles as the API path.
fn mcp_tool_to_method(name: &str) -> &str {
    match name {
        // Navigation
        "browser_navigate" => "browserlane:page.navigate",
        "browser_back" => "browserlane:page.back",
        "browser_forward" => "browserlane:page.forward",
        "browser_reload" => "browserlane:page.reload",
        // Element interaction
        "browser_click" => "browserlane:element.click",
        "browser_dblclick" => "browserlane:element.dblclick",
        "browser_fill" => "browserlane:element.fill",
        "browser_type" => "browserlane:element.type",
        "browser_press" => "browserlane:element.press",
        "browser_hover" => "browserlane:element.hover",
        "browser_select" => "browserlane:element.selectOption",
        "browser_check" => "browserlane:element.check",
        "browser_uncheck" => "browserlane:element.uncheck",
        "browser_focus" => "browserlane:element.focus",
        "browser_scroll_into_view" => "browserlane:element.scrollIntoView",
        "browser_drag" => "browserlane:element.dragTo",
        // Keyboard/mouse
        "browser_keys" => "browserlane:keyboard.press",
        "browser_mouse_move" => "browserlane:mouse.move",
        "browser_mouse_down" => "browserlane:mouse.down",
        "browser_mouse_up" => "browserlane:mouse.up",
        "browser_mouse_click" => "browserlane:mouse.click",
        "browser_scroll" => "browserlane:page.scroll",
        // Page queries
        "browser_find" => "browserlane:page.find",
        "browser_find_all" => "browserlane:page.findAll",
        "browser_get_text" => "browserlane:element.text",
        "browser_get_html" => "browserlane:element.html",
        "browser_get_url" => "browserlane:page.url",
        "browser_get_title" => "browserlane:page.title",
        "browser_get_value" => "browserlane:element.value",
        "browser_get_attribute" => "browserlane:element.attr",
        "browser_is_visible" => "browserlane:element.isVisible",
        "browser_is_enabled" => "browserlane:element.isEnabled",
        "browser_is_checked" => "browserlane:element.isChecked",
        "browser_count" => "browserlane:page.findAll",
        "browser_evaluate" => "browserlane:page.eval",
        "browser_screenshot" => "browserlane:page.screenshot",
        "browser_pdf" => "browserlane:page.pdf",
        "browser_a11y_tree" => "browserlane:page.a11yTree",
        // Waiting
        "browser_wait" => "browserlane:page.waitFor",
        "browser_wait_for_url" => "browserlane:page.waitForURL",
        "browser_wait_for_load" => "browserlane:page.waitForLoad",
        "browser_wait_for_text" => "browserlane:page.wait",
        "browser_wait_for_fn" => "browserlane:page.waitForFunction",
        "browser_sleep" => "browserlane:page.wait",
        // Pages
        "browser_new_page" => "browserlane:browser.newPage",
        "browser_list_pages" => "browserlane:browser.pages",
        "browser_switch_page" => "browserlane:page.activate",
        "browser_close_page" => "browserlane:page.close",
        // Viewport/window
        "browser_set_viewport" => "browserlane:page.setViewport",
        "browser_get_viewport" => "browserlane:page.viewport",
        "browser_set_window" => "browserlane:page.setWindow",
        "browser_get_window" => "browserlane:page.window",
        // Cookies/storage
        "browser_get_cookies" => "browserlane:context.cookies",
        "browser_set_cookie" => "browserlane:context.setCookies",
        "browser_delete_cookies" => "browserlane:context.clearCookies",
        "browser_storage_state" => "browserlane:context.storage",
        "browser_restore_storage" => "browserlane:context.setStorage",
        // Dialog
        "browser_dialog_accept" => "browserlane:dialog.accept",
        "browser_dialog_dismiss" => "browserlane:dialog.dismiss",
        // Media/content
        "browser_emulate_media" => "browserlane:page.emulateMedia",
        "browser_set_geolocation" => "browserlane:page.setGeolocation",
        "browser_set_content" => "browserlane:page.setContent",
        // Frames
        "browser_frames" => "browserlane:page.frames",
        "browser_frame" => "browserlane:page.frame",
        // Upload/download
        "browser_upload" => "browserlane:element.setFiles",
        "browser_download_set_dir" => "browserlane:download.saveAs",
        // Browser lifecycle
        "browser_start" => "browserlane:browser.newPage",
        "browser_stop" => "browserlane:browser.stop",
        // Map/highlight (browserlane-specific)
        "browser_map" => "browserlane:page.eval",
        "browser_diff_map" => "browserlane:page.eval",
        "browser_highlight" => "browserlane:page.eval",
        // Clock
        "page_clock_install" => "browserlane:clock.install",
        "page_clock_fast_forward" => "browserlane:clock.fastForward",
        "page_clock_run_for" => "browserlane:clock.runFor",
        "page_clock_pause_at" => "browserlane:clock.pauseAt",
        "page_clock_resume" => "browserlane:clock.resume",
        "page_clock_set_fixed_time" => "browserlane:clock.setFixedTime",
        "page_clock_set_system_time" => "browserlane:clock.setSystemTime",
        "page_clock_set_timezone" => "browserlane:clock.setTimezone",
        _ => name,
    }
}

/// Formats a JSON value the way Go's `fmt` default verb `%v` would, so
/// `browser_evaluate` output stays byte-identical to the Go binary:
/// - slices render as `[a b c]` (space-separated, no commas/quotes),
/// - maps render as `map[k:v k2:v2]` with keys sorted alphabetically,
/// - applied recursively; a nil interface renders as `<nil>`.
fn go_fmt_v(v: &Value) -> String {
    match v {
        Value::Null => "<nil>".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(go_fmt_v).collect();
            format!("[{}]", parts.join(" "))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|k| format!("{}:{}", k, go_fmt_v(&map[*k])))
                .collect();
            format!("map[{}]", parts.join(" "))
        }
    }
}

/// Builds an image ToolsCallResult (base64 data + mime type).
fn image_result(data: &str, mime_type: &str) -> ToolsCallResult {
    ToolsCallResult {
        content: vec![Content {
            content_type: "image".to_string(),
            text: String::new(),
            data: data.to_string(),
            mime_type: mime_type.to_string(),
        }],
        is_error: false,
    }
}

/// Builds a text ToolsCallResult.
fn text_result(text: &str) -> ToolsCallResult {
    ToolsCallResult {
        content: vec![Content {
            content_type: "text".to_string(),
            text: text.to_string(),
            data: String::new(),
            mime_type: String::new(),
        }],
        is_error: false,
    }
}

/// Builds an error ToolsCallResult (is_error=true, no Err return).
fn error_result(text: &str) -> ToolsCallResult {
    ToolsCallResult {
        content: vec![Content {
            content_type: "text".to_string(),
            text: text.to_string(),
            data: String::new(),
            mime_type: String::new(),
        }],
        is_error: true,
    }
}

/// A {selector, label} pair returned by the find scripts.
#[derive(Debug, Default, Deserialize)]
struct FindResult {
    #[serde(default)]
    selector: String,
    #[serde(default)]
    label: String,
}

/// Extracts a float arg, accepting numbers or numeric strings.
fn arg_float(args: &Map<String, Value>, key: &str) -> Option<f64> {
    match args.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Polls a JS function until it returns a non-empty result or times out.
async fn poll_call_function(
    client: &Arc<bidi::Client>,
    script: &str,
    args: Vec<Value>,
    timeout: Duration,
) -> anyhow::Result<Value> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Ok(result) = client.call_function("", script, args.clone()).await {
            if !result.is_null() {
                match result.as_str() {
                    Some(s) if s.is_empty() || s == "null" => {}
                    _ => return Ok(result),
                }
            }
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!(
                "timeout after {}",
                crate::errors::format_go_duration(timeout)
            ));
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// JS getSelector(el) — generates unique CSS selectors.
/// JS that finds interactive elements and returns `[{selector, label}]`.
fn map_script() -> String {
    let body = r#"(scopeSelector) => {
		__GET_SELECTOR_JS__
		__GET_LABEL_JS__

		const interactive = 'a[href], button, input, textarea, select, [role="button"], [role="link"], [role="checkbox"], [role="radio"], [role="tab"], [role="menuitem"], [role="switch"], [onclick], [tabindex]:not([tabindex="-1"]), summary, details';

		const root = scopeSelector ? document.querySelector(scopeSelector) : document;
		if (!root) return JSON.stringify([]);
		const els = root.querySelectorAll(interactive);
		const results = [];
		const seen = new Set();

		for (const el of els) {
			const style = window.getComputedStyle(el);
			if (style.display === 'none' || style.visibility === 'hidden' || el.offsetWidth === 0) continue;

			const sel = getSelector(el);
			if (seen.has(sel)) continue;
			seen.add(sel);

			results.push({ selector: sel, label: getLabel(el) });
		}

		return JSON.stringify(results);
	}"#;
    body.replace("__GET_SELECTOR_JS__", get_selector_js())
        .replace("__GET_LABEL_JS__", get_label_js())
}

fn get_selector_js() -> &'static str {
    r##"function getSelector(el) {
			if (el.id) return '#' + CSS.escape(el.id);
			const parts = [];
			let cur = el;
			while (cur && cur !== document.body && cur !== document.documentElement) {
				let seg = cur.tagName.toLowerCase();
				if (cur.id) {
					parts.unshift('#' + CSS.escape(cur.id));
					break;
				}
				const parent = cur.parentElement;
				if (parent) {
					const siblings = Array.from(parent.children).filter(c => c.tagName === cur.tagName);
					if (siblings.length > 1) {
						const idx = siblings.indexOf(cur) + 1;
						seg += ':nth-of-type(' + idx + ')';
					}
				}
				parts.unshift(seg);
				cur = parent;
			}
			if (parts.length === 0) return el.tagName.toLowerCase();
			if (!parts[0].startsWith('#')) parts.unshift('body');
			return parts.join(' > ');
		}"##
}

/// JS getLabel(el) — generates descriptive `[tag] "text"` labels.
fn get_label_js() -> &'static str {
    r##"function getLabel(el) {
			const tag = el.tagName.toLowerCase();
			const type = el.getAttribute('type');
			let desc = '[' + tag;
			if (type) desc += ' type="' + type + '"';
			desc += ']';

			const ariaLabel = el.getAttribute('aria-label');
			if (ariaLabel) return desc + ' "' + ariaLabel.substring(0, 60) + '"';

			const placeholder = el.getAttribute('placeholder');
			if (placeholder) return desc + ' placeholder="' + placeholder.substring(0, 60) + '"';

			const title = el.getAttribute('title');
			if (title) return desc + ' title="' + title.substring(0, 60) + '"';

			const text = (el.textContent || '').trim().substring(0, 60);
			if (text) return desc + ' "' + text + '"';

			const name = el.getAttribute('name');
			if (name) return desc + ' name="' + name + '"';

			const src = el.getAttribute('src');
			if (src) return desc + ' src="' + src.substring(0, 60) + '"';

			return desc;
		}"##
}

/// JS function for finding elements by semantic criteria.
/// Returns JSON: {"selector":"...","label":"...","tag":"...","text":"...","box":{...}}
fn find_by_semantic_script() -> String {
    let mut s = String::from("(role, text, label, placeholder, testid, xpath, alt, title) => {\n\t\t");
    s.push_str(get_selector_js());
    s.push_str("\n\t\t");
    s.push_str(get_label_js());
    s.push_str(SEMANTIC_FIND_REST);
    s
}

const SEMANTIC_FIND_REST: &str = r##"

		const IMPLICIT_ROLES = {
			A: (el) => el.hasAttribute('href') ? 'link' : '',
			AREA: (el) => el.hasAttribute('href') ? 'link' : '',
			ARTICLE: () => 'article',
			ASIDE: () => 'complementary',
			BUTTON: () => 'button',
			DETAILS: () => 'group',
			DIALOG: () => 'dialog',
			FOOTER: () => 'contentinfo',
			FORM: () => 'form',
			H1: () => 'heading', H2: () => 'heading', H3: () => 'heading',
			H4: () => 'heading', H5: () => 'heading', H6: () => 'heading',
			HEADER: () => 'banner',
			HR: () => 'separator',
			IMG: (el) => el.getAttribute('alt') ? 'img' : 'presentation',
			INPUT: (el) => {
				const t = (el.getAttribute('type') || 'text').toLowerCase();
				const map = {button:'button',checkbox:'checkbox',image:'button',
					number:'spinbutton',radio:'radio',range:'slider',
					reset:'button',search:'searchbox',submit:'button',text:'textbox',
					email:'textbox',tel:'textbox',url:'textbox',password:'textbox'};
				return map[t] || 'textbox';
			},
			LI: () => 'listitem',
			MAIN: () => 'main',
			MENU: () => 'list',
			NAV: () => 'navigation',
			OL: () => 'list',
			OPTION: () => 'option',
			OUTPUT: () => 'status',
			PROGRESS: () => 'progressbar',
			SECTION: () => 'region',
			SELECT: (el) => el.hasAttribute('multiple') ? 'listbox' : 'combobox',
			SUMMARY: () => 'button',
			TABLE: () => 'table',
			TBODY: () => 'rowgroup', THEAD: () => 'rowgroup', TFOOT: () => 'rowgroup',
			TD: () => 'cell',
			TEXTAREA: () => 'textbox',
			TH: () => 'columnheader',
			TR: () => 'row',
			UL: () => 'list',
		};

		function getImplicitRole(el) {
			const explicit = el.getAttribute('role');
			if (explicit) return explicit.toLowerCase();
			const fn = IMPLICIT_ROLES[el.tagName];
			return fn ? fn(el).toLowerCase() : '';
		}

		function getName(el) {
			const ariaLabel = el.getAttribute('aria-label');
			if (ariaLabel) return ariaLabel;
			const labelledBy = el.getAttribute('aria-labelledby');
			if (labelledBy) {
				const parts = labelledBy.split(/\s+/).map(id => {
					const ref = document.getElementById(id);
					return ref ? (ref.textContent || '').trim() : '';
				}).filter(Boolean);
				if (parts.length) return parts.join(' ');
			}
			if (el.id) {
				const assocLabel = document.querySelector('label[for="' + el.id + '"]');
				if (assocLabel) return (assocLabel.textContent || '').trim();
			}
			const ph = el.getAttribute('placeholder');
			if (ph) return ph;
			const altAttr = el.getAttribute('alt');
			if (altAttr) return altAttr;
			const titleAttr = el.getAttribute('title');
			if (titleAttr) return titleAttr;
			return (el.textContent || '').trim();
		}

		let el = null;

		if (role) {
			const roleLower = role.toLowerCase();
			const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT);
			const found = [];
			let node;
			while (node = walker.nextNode()) {
				if (getImplicitRole(node) !== roleLower) continue;
				if (text && !(node.textContent || '').trim().includes(text)) continue;
				if (label) {
					const elName = getName(node);
					if (!elName.includes(label)) continue;
				}
				if (placeholder) {
					const ph = node.getAttribute('placeholder');
					if (!ph || !ph.includes(placeholder)) continue;
				}
				if (testid) {
					const tid = node.getAttribute('data-testid');
					if (tid !== testid) continue;
				}
				if (alt) {
					const a = node.getAttribute('alt');
					if (!a || !a.includes(alt)) continue;
				}
				if (title) {
					const t = node.getAttribute('title');
					if (!t || !t.includes(title)) continue;
				}
				found.push(node);
			}
			if (found.length === 0) return null;
			el = found[0];
			if (text && found.length > 1) {
				let bestLen = (el.textContent || '').length;
				for (let i = 1; i < found.length; i++) {
					const len = (found[i].textContent || '').length;
					if (len < bestLen) { el = found[i]; bestLen = len; }
				}
			}
		} else if (xpath) {
			const xresult = document.evaluate(xpath, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
			el = xresult.singleNodeValue;
		} else if (testid) {
			el = document.querySelector('[data-testid="' + testid.replace(/"/g, '\\"') + '"]');
		} else if (placeholder) {
			el = document.querySelector('[placeholder="' + placeholder.replace(/"/g, '\\"') + '"]');
		} else if (alt) {
			el = document.querySelector('[alt="' + alt.replace(/"/g, '\\"') + '"]');
		} else if (title) {
			el = document.querySelector('[title="' + title.replace(/"/g, '\\"') + '"]');
		} else if (label) {
			const labels = document.querySelectorAll('label');
			for (const lbl of labels) {
				if (lbl.textContent.trim().includes(label)) {
					if (lbl.htmlFor) {
						el = document.getElementById(lbl.htmlFor);
					} else {
						el = lbl.querySelector('input, textarea, select');
					}
					if (el) break;
				}
			}
			if (!el) {
				el = document.querySelector('[aria-label="' + label.replace(/"/g, '\\"') + '"]');
			}
			if (!el) {
				const all = document.querySelectorAll('[aria-labelledby]');
				for (const candidate of all) {
					const labelId = candidate.getAttribute('aria-labelledby');
					const labelEl = document.getElementById(labelId);
					if (labelEl && labelEl.textContent.trim().includes(label)) {
						el = candidate;
						break;
					}
				}
			}
		} else if (text) {
			const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, {
				acceptNode: (node) => {
					if (node.offsetWidth === 0 && node.offsetHeight === 0) return NodeFilter.FILTER_REJECT;
					const style = window.getComputedStyle(node);
					if (style.display === 'none' || style.visibility === 'hidden') return NodeFilter.FILTER_REJECT;
					return NodeFilter.FILTER_ACCEPT;
				}
			});
			let best = null;
			let bestLen = Infinity;
			let node;
			while (node = walker.nextNode()) {
				const content = node.textContent.trim();
				if (content.includes(text) && content.length < bestLen) {
					best = node;
					bestLen = content.length;
				}
			}
			el = best;
		}

		if (!el) return null;

		if (el.scrollIntoViewIfNeeded) {
			el.scrollIntoViewIfNeeded(true);
		} else {
			el.scrollIntoView({ block: 'center', inline: 'nearest' });
		}

		const rect = el.getBoundingClientRect();
		return JSON.stringify({
			selector: getSelector(el),
			label: getLabel(el),
			tag: el.tagName.toLowerCase(),
			text: (el.textContent || '').trim().substring(0, 100),
			box: { x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height) }
		});
	}"##;
