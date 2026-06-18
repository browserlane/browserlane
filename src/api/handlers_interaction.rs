//! Phase 3 interaction cluster: the element-action routes (click / dblclick /
//! tap / hover / focus / fill / type / press / clear / check / uncheck /
//! selectOption / dragTo / scrollIntoView / dispatchEvent / setFiles), their
//! exported standalone primitives + composite functions, and the JS script
//! builders. Click-like and fill-like handlers call
//! capture_before_snapshot_after_scroll between resolve and act (the recorder
//! injects `_recordCallId` via dispatch).

use std::sync::Arc;

use anyhow::anyhow;
use serde_json::{json, Map, Value};

use super::actionability::{
    click_checks, fill_checks, hover_checks, resolve_with_actionability, scroll_checks,
    select_checks,
};
use super::helpers::{
    call_script, extract_element_params, parse_script_result, resolve_element, resolve_element_ref,
    ElementInfo, ElementParams,
};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::{new_api_session, Session};
use crate::bidi;

// ---------------------------------------------------------------------------
// Router routes — element-action handlers.
// ---------------------------------------------------------------------------

impl Router {
    /// Handles `browserlane:element.click` with actionability checks.
    pub(crate) async fn handle_browserlane_click(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep, &click_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        if let Err(e) = click_at_center(&s, &context, &info).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "clicked": true }));
    }

    /// Handles `browserlane:element.dblclick`.
    pub(crate) async fn handle_browserlane_dblclick(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep, &click_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        if let Err(e) = dbl_click_at_center(&s, &context, &info).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "dblclicked": true }));
    }

    /// Handles `browserlane:element.fill` — sets the value via JS + input/change events.
    pub(crate) async fn handle_browserlane_fill(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let value = cmd.params.get("value").and_then(Value::as_str).unwrap_or("").to_string();
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = resolve_with_actionability(&s, &context, ep.clone(), &fill_checks()).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        let (script, args) = build_set_value_script(&ep, &value);
        let resp = match call_script(&s, &context, &script, args).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let val = match parse_script_result(&resp) {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("fill failed: {e}")),
        };
        if val != "ok" {
            return self.send_error(session, cmd.id, &anyhow!("fill: {val}"));
        }
        self.send_success(session, cmd.id, json!({ "filled": true }));
    }

    /// Handles `browserlane:element.type` — clicks to focus, then types (no clear).
    pub(crate) async fn handle_browserlane_type(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        // Extract text-to-type BEFORE building element params, since "text" is
        // also a semantic selector param. Remove it to avoid collision.
        let text = cmd.params.get("text").and_then(Value::as_str).unwrap_or("").to_string();
        let mut params_copy = Map::new();
        for (k, v) in &cmd.params {
            if k != "text" {
                params_copy.insert(k.clone(), v.clone());
            }
        }
        let ep = extract_element_params(&params_copy);

        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep, &click_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        if let Err(e) = click_at_center(&s, &context, &info).await {
            return self.send_error(session, cmd.id, &e);
        }
        if let Err(e) = type_text(&s, &context, &text).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "typed": true }));
    }

    /// Handles `browserlane:element.press` — clicks to focus, then presses a key/combo.
    pub(crate) async fn handle_browserlane_press(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let key = cmd.params.get("key").and_then(Value::as_str).unwrap_or("").to_string();
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep, &click_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        if let Err(e) = click_at_center(&s, &context, &info).await {
            return self.send_error(session, cmd.id, &e);
        }
        if let Err(e) = press_key(&s, &context, &key).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "pressed": true }));
    }

    /// Handles `browserlane:element.clear` — clears the value via JS.
    pub(crate) async fn handle_browserlane_clear(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = resolve_with_actionability(&s, &context, ep.clone(), &fill_checks()).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        let (script, args) = build_set_value_script(&ep, "");
        let resp = match call_script(&s, &context, &script, args).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let val = match parse_script_result(&resp) {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("clear failed: {e}")),
        };
        if val != "ok" {
            return self.send_error(session, cmd.id, &anyhow!("clear: {val}"));
        }
        self.send_success(session, cmd.id, json!({ "cleared": true }));
    }

    /// Handles `browserlane:element.check` — clicks the box only if not already checked.
    pub(crate) async fn handle_browserlane_check(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep.clone(), &click_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        let checked = match is_checked(&s, &context, ep).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if !checked {
            if let Err(e) = click_at_center(&s, &context, &info).await {
                return self.send_error(session, cmd.id, &e);
            }
        }
        self.send_success(session, cmd.id, json!({ "checked": true }));
    }

    /// Handles `browserlane:element.uncheck` — clicks the box only if currently checked.
    pub(crate) async fn handle_browserlane_uncheck(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep.clone(), &click_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        let checked = match is_checked(&s, &context, ep).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if checked {
            if let Err(e) = click_at_center(&s, &context, &info).await {
                return self.send_error(session, cmd.id, &e);
            }
        }
        self.send_success(session, cmd.id, json!({ "unchecked": true }));
    }

    /// Handles `browserlane:element.selectOption` — sets a <select> value + change event.
    pub(crate) async fn handle_browserlane_select_option(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let value = cmd.params.get("value").and_then(Value::as_str).unwrap_or("").to_string();
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = resolve_with_actionability(&s, &context, ep.clone(), &select_checks()).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        let (script, args) = build_select_option_script(&ep, &value);
        let resp = match call_script(&s, &context, &script, args).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let val = match parse_script_result(&resp) {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("selectOption failed: {e}")),
        };
        if val != "ok" {
            return self.send_error(session, cmd.id, &anyhow!("selectOption: {val}"));
        }
        self.send_success(session, cmd.id, json!({ "selected": true }));
    }

    /// Handles `browserlane:element.hover` — moves the pointer to the element center.
    pub(crate) async fn handle_browserlane_hover(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep, &hover_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        if let Err(e) = hover_at_center(&s, &context, &info).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "hovered": true }));
    }

    /// Handles `browserlane:element.focus` — runs element.focus() via JS.
    pub(crate) async fn handle_browserlane_focus(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = focus_element(&s, &context, ep).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "focused": true }));
    }

    /// Handles `browserlane:element.dragTo` — drags from source to target element.
    pub(crate) async fn handle_browserlane_drag_to(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let target_params = match cmd.params.get("target").and_then(Value::as_object) {
            Some(t) => t.clone(),
            None => return self.send_error(session, cmd.id, &anyhow!("dragTo requires 'target' parameter")),
        };
        let target_ep = extract_element_params(&target_params);

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let src_info = match resolve_with_actionability(&s, &context, ep, &hover_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("source: {e}")),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        let target_info = match resolve_with_actionability(&s, &context, target_ep, &hover_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("target: {e}")),
        };

        let src_x = (src_info.box_.x + src_info.box_.width / 2.0) as i64;
        let src_y = (src_info.box_.y + src_info.box_.height / 2.0) as i64;
        let dst_x = (target_info.box_.x + target_info.box_.width / 2.0) as i64;
        let dst_y = (target_info.box_.y + target_info.box_.height / 2.0) as i64;

        let drag_params = json!({
            "context": context,
            "actions": [{
                "type": "pointer",
                "id": "mouse",
                "parameters": { "pointerType": "mouse" },
                "actions": [
                    {"type": "pointerMove", "x": src_x, "y": src_y, "duration": 0},
                    {"type": "pointerDown", "button": 0},
                    {"type": "pause", "duration": 100},
                    {"type": "pointerMove", "x": dst_x, "y": dst_y, "duration": 200},
                    {"type": "pointerUp", "button": 0},
                ],
            }],
        });

        if let Err(e) = s.send_bidi_command("input.performActions", drag_params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "dragged": true }));
    }

    /// Handles `browserlane:element.tap` — performs a touch tap at the element center.
    pub(crate) async fn handle_browserlane_tap(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_with_actionability(&s, &context, ep, &click_checks()).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        self.capture_before_snapshot_after_scroll(session, &cmd.params).await;
        if let Err(e) = tap_at_center(&s, &context, &info).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "tapped": true }));
    }

    /// Handles `browserlane:element.scrollIntoView` — resolves (auto-scrolls into view).
    pub(crate) async fn handle_browserlane_scroll_into_view(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = scroll_into_view(&s, &context, ep).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "scrolled": true }));
    }

    /// Handles `browserlane:element.dispatchEvent` — dispatches a DOM event via JS.
    pub(crate) async fn handle_browserlane_dispatch_event(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let event_type = cmd.params.get("eventType").and_then(Value::as_str).unwrap_or("").to_string();
        let init_json = match cmd.params.get("eventInit") {
            Some(v) if v.is_object() => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
            _ => "{}".to_string(),
        };

        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        // Resolve element to confirm it exists.
        if let Err(e) = resolve_element(&s, &context, ep.clone()).await {
            return self.send_error(session, cmd.id, &e);
        }

        let (script, args) = build_dispatch_event_script(&ep, &event_type, &init_json);
        let params = json!({
            "functionDeclaration": script,
            "target": { "context": context },
            "arguments": args,
            "awaitPromise": false,
            "resultOwnership": "root",
        });
        if let Err(e) = self.send_internal_command(session, "script.callFunction", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "dispatched": true }));
    }

    /// Handles `browserlane:element.setFiles` — sets files on an <input type="file">.
    pub(crate) async fn handle_browserlane_el_set_files(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let files_raw = match cmd.params.get("files") {
            Some(v) => v,
            None => return self.send_error(session, cmd.id, &anyhow!("el.setFiles requires 'files' parameter")),
        };
        let files_arr = match files_raw.as_array() {
            Some(a) => a,
            None => return self.send_error(session, cmd.id, &anyhow!("el.setFiles: 'files' must be an array")),
        };
        let mut files: Vec<String> = Vec::with_capacity(files_arr.len());
        for f in files_arr {
            match f.as_str() {
                Some(s) => files.push(s.to_string()),
                None => return self.send_error(session, cmd.id, &anyhow!("el.setFiles: each file must be a string")),
            }
        }

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let shared_id = match resolve_element_ref(&s, &context, ep).await {
            Ok(id) => id,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let params = json!({
            "context": context,
            "element": { "sharedId": shared_id },
            "files": files,
        });
        if let Err(e) = self.send_internal_command(session, "input.setFiles", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "set": true }));
    }
}

// ---------------------------------------------------------------------------
// Exported standalone input primitives — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Performs a mouse click at the center of an element.
pub async fn click_at_center(s: &dyn Session, context: &str, info: &ElementInfo) -> anyhow::Result<()> {
    let x = (info.box_.x + info.box_.width / 2.0) as i64;
    let y = (info.box_.y + info.box_.height / 2.0) as i64;
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [
                {"type": "pointerMove", "x": x, "y": y, "duration": 0},
                {"type": "pointerDown", "button": 0},
                {"type": "pointerUp", "button": 0},
            ],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Performs a double-click at the center of an element.
pub async fn dbl_click_at_center(s: &dyn Session, context: &str, info: &ElementInfo) -> anyhow::Result<()> {
    let x = (info.box_.x + info.box_.width / 2.0) as i64;
    let y = (info.box_.y + info.box_.height / 2.0) as i64;
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [
                {"type": "pointerMove", "x": x, "y": y, "duration": 0},
                {"type": "pointerDown", "button": 0},
                {"type": "pointerUp", "button": 0},
                {"type": "pointerDown", "button": 0},
                {"type": "pointerUp", "button": 0},
            ],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Types a string of text using keyboard events.
pub async fn type_text(s: &dyn Session, context: &str, text: &str) -> anyhow::Result<()> {
    let mut key_actions: Vec<Value> = Vec::with_capacity(text.chars().count() * 2);
    for ch in text.chars() {
        let c = ch.to_string();
        key_actions.push(json!({ "type": "keyDown", "value": c }));
        key_actions.push(json!({ "type": "keyUp", "value": c }));
    }
    let params = json!({
        "context": context,
        "actions": [{ "type": "key", "id": "keyboard", "actions": key_actions }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Presses a key or key combo (e.g. "Enter", "Control+a").
pub async fn press_key(s: &dyn Session, context: &str, key: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = key.split('+').collect();
    let mut key_actions: Vec<Value> = Vec::new();

    if parts.len() == 1 {
        let resolved = bidi::resolve_key(parts[0]);
        key_actions.push(json!({ "type": "keyDown", "value": resolved }));
        key_actions.push(json!({ "type": "keyUp", "value": resolved }));
    } else {
        for part in &parts[..parts.len() - 1] {
            key_actions.push(json!({ "type": "keyDown", "value": bidi::resolve_key(part.trim()) }));
        }
        let main_key = bidi::resolve_key(parts[parts.len() - 1].trim());
        key_actions.push(json!({ "type": "keyDown", "value": main_key }));
        key_actions.push(json!({ "type": "keyUp", "value": main_key }));
        for part in parts[..parts.len() - 1].iter().rev() {
            key_actions.push(json!({ "type": "keyUp", "value": bidi::resolve_key(part.trim()) }));
        }
    }

    let params = json!({
        "context": context,
        "actions": [{ "type": "key", "id": "keyboard", "actions": key_actions }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Moves the mouse to the center of an element without clicking.
pub async fn hover_at_center(s: &dyn Session, context: &str, info: &ElementInfo) -> anyhow::Result<()> {
    let x = (info.box_.x + info.box_.width / 2.0) as i64;
    let y = (info.box_.y + info.box_.height / 2.0) as i64;
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [{"type": "pointerMove", "x": x, "y": y, "duration": 0}],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Performs a touch tap at the center of an element.
pub async fn tap_at_center(s: &dyn Session, context: &str, info: &ElementInfo) -> anyhow::Result<()> {
    let x = (info.box_.x + info.box_.width / 2.0) as i64;
    let y = (info.box_.y + info.box_.height / 2.0) as i64;
    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "touch",
            "parameters": { "pointerType": "touch" },
            "actions": [
                {"type": "pointerMove", "x": x, "y": y, "duration": 0},
                {"type": "pointerDown", "button": 0},
                {"type": "pointerUp", "button": 0},
            ],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

/// Performs a mouse wheel scroll at the given coordinates.
pub async fn scroll_wheel(
    s: &dyn Session,
    context: &str,
    x: i64,
    y: i64,
    delta_x: i64,
    delta_y: i64,
) -> anyhow::Result<()> {
    let params = json!({
        "context": context,
        "actions": [{
            "type": "wheel",
            "id": "wheel",
            "actions": [{
                "type": "scroll",
                "x": x,
                "y": y,
                "deltaX": delta_x,
                "deltaY": delta_y,
            }],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

// ---------------------------------------------------------------------------
// Exported standalone composite functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Resolves an element with actionability checks and clicks at its center.
pub async fn click(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    let info = resolve_with_actionability(s, context, ep, &click_checks()).await?;
    click_at_center(s, context, &info).await
}

/// Resolves an element with actionability checks and double-clicks at its center.
pub async fn dbl_click(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    let info = resolve_with_actionability(s, context, ep, &click_checks()).await?;
    dbl_click_at_center(s, context, &info).await
}

/// Resolves an element with actionability checks and moves the mouse to its center.
pub async fn hover(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    let info = resolve_with_actionability(s, context, ep, &hover_checks()).await?;
    hover_at_center(s, context, &info).await
}

/// Resolves an element with actionability checks and sets its value via JS.
pub async fn fill(s: &dyn Session, context: &str, ep: ElementParams, value: &str) -> anyhow::Result<()> {
    resolve_with_actionability(s, context, ep.clone(), &fill_checks()).await?;
    let (script, args) = build_set_value_script(&ep, value);
    let resp = call_script(s, context, &script, args).await?;
    let val = parse_script_result(&resp).map_err(|e| anyhow!("fill failed: {e}"))?;
    if val != "ok" {
        return Err(anyhow!("fill: {val}"));
    }
    Ok(())
}

/// Resolves an element with actionability checks, clicks to focus, and types text.
pub async fn type_into(s: &dyn Session, context: &str, ep: ElementParams, text: &str) -> anyhow::Result<()> {
    let info = resolve_with_actionability(s, context, ep, &click_checks()).await?;
    click_at_center(s, context, &info).await?;
    type_text(s, context, text).await
}

/// Resolves an element with actionability checks, clicks to focus, and presses a key.
pub async fn press_on(s: &dyn Session, context: &str, ep: ElementParams, key: &str) -> anyhow::Result<()> {
    let info = resolve_with_actionability(s, context, ep, &click_checks()).await?;
    click_at_center(s, context, &info).await?;
    press_key(s, context, key).await
}

/// Resolves a checkbox with actionability checks and clicks it only if not already
/// checked. Returns true if it was toggled.
pub async fn check(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<bool> {
    let info = resolve_with_actionability(s, context, ep.clone(), &click_checks()).await?;
    let checked = is_checked(s, context, ep).await?;
    if !checked {
        click_at_center(s, context, &info).await?;
        return Ok(true);
    }
    Ok(false)
}

/// Resolves a checkbox with actionability checks and clicks it only if currently
/// checked. Returns true if it was toggled.
pub async fn uncheck(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<bool> {
    let info = resolve_with_actionability(s, context, ep.clone(), &click_checks()).await?;
    let checked = is_checked(s, context, ep).await?;
    if checked {
        click_at_center(s, context, &info).await?;
        return Ok(true);
    }
    Ok(false)
}

/// Checks if a checkbox/radio element is checked.
pub async fn is_checked(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<bool> {
    let (script, args) = build_is_checked_script(&ep);
    let resp = call_script(s, context, &script, args).await?;
    let val = parse_script_result(&resp)?;
    Ok(val == "true")
}

/// Resolves a select element with actionability checks and sets its value.
pub async fn select_option(s: &dyn Session, context: &str, ep: ElementParams, value: &str) -> anyhow::Result<()> {
    resolve_with_actionability(s, context, ep.clone(), &select_checks()).await?;
    let (script, args) = build_select_option_script(&ep, value);
    let resp = call_script(s, context, &script, args).await?;
    let val = parse_script_result(&resp).map_err(|e| anyhow!("selectOption failed: {e}"))?;
    if val != "ok" {
        return Err(anyhow!("selectOption: {val}"));
    }
    Ok(())
}

/// Resolves an element and focuses it via JS.
pub async fn focus_element(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    resolve_element(s, context, ep.clone()).await?;
    let (script, args) = build_focus_script(&ep);
    call_script(s, context, &script, args).await.map(|_| ())
}

/// Resolves an element with a stability check, which auto-scrolls it into view.
pub async fn scroll_into_view(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    resolve_with_actionability(s, context, ep, &scroll_checks()).await.map(|_| ())
}

/// Resolves an element with actionability checks and performs a touch tap at its center.
pub async fn tap(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    let info = resolve_with_actionability(s, context, ep, &click_checks()).await?;
    tap_at_center(s, context, &info).await
}

/// Resolves source and target elements with actionability checks and drags between them.
pub async fn drag_to(
    s: &dyn Session,
    context: &str,
    source: ElementParams,
    target: ElementParams,
) -> anyhow::Result<()> {
    let src_info = resolve_with_actionability(s, context, source, &hover_checks())
        .await
        .map_err(|e| anyhow!("source: {e}"))?;
    let target_info = resolve_with_actionability(s, context, target, &hover_checks())
        .await
        .map_err(|e| anyhow!("target: {e}"))?;

    let src_x = (src_info.box_.x + src_info.box_.width / 2.0) as i64;
    let src_y = (src_info.box_.y + src_info.box_.height / 2.0) as i64;
    let dst_x = (target_info.box_.x + target_info.box_.width / 2.0) as i64;
    let dst_y = (target_info.box_.y + target_info.box_.height / 2.0) as i64;

    let params = json!({
        "context": context,
        "actions": [{
            "type": "pointer",
            "id": "mouse",
            "parameters": { "pointerType": "mouse" },
            "actions": [
                {"type": "pointerMove", "x": src_x, "y": src_y, "duration": 0},
                {"type": "pointerDown", "button": 0},
                {"type": "pause", "duration": 100},
                {"type": "pointerMove", "x": dst_x, "y": dst_y, "duration": 200},
                {"type": "pointerUp", "button": 0},
            ],
        }],
    });
    s.send_bidi_command("input.performActions", params).await.map(|_| ())
}

// ---------------------------------------------------------------------------
// Script builders for JS-based interactions.
// ---------------------------------------------------------------------------

/// Builds the JS function that resolves an element and returns its checked state.
fn build_is_checked_script(ep: &ElementParams) -> (String, Vec<Value>) {
    let args = vec![
        json!({ "type": "string", "value": ep.scope }),
        json!({ "type": "string", "value": ep.selector }),
        json!({ "type": "number", "value": ep.index }),
        json!({ "type": "boolean", "value": ep.has_index }),
    ];
    let script = r#"
		(scope, selector, index, hasIndex) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return 'false';
			let el;
			if (hasIndex) {
				const all = root.querySelectorAll(selector);
				el = all[index];
			} else {
				el = root.querySelector(selector);
			}
			if (!el) return 'false';
			return el.checked ? 'true' : 'false';
		}
	"#
    .to_string();
    (script, args)
}

/// Builds a JS function to set a select element's value.
fn build_select_option_script(ep: &ElementParams, value: &str) -> (String, Vec<Value>) {
    let args = vec![
        json!({ "type": "string", "value": ep.scope }),
        json!({ "type": "string", "value": ep.selector }),
        json!({ "type": "number", "value": ep.index }),
        json!({ "type": "boolean", "value": ep.has_index }),
        json!({ "type": "string", "value": value }),
    ];
    let script = r#"
		(scope, selector, index, hasIndex, value) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return 'element not found';
			let el;
			if (hasIndex) {
				const all = root.querySelectorAll(selector);
				el = all[index];
			} else {
				el = root.querySelector(selector);
			}
			if (!el) return 'element not found';
			if (el.tagName !== 'SELECT') return 'not a <select> element';
			// Match by option value, then fall back to the visible label/text so
			// passing "California" (for <option value="CA">California</option>)
			// works. Without this, an unmatched value silently no-ops (issue #140).
			const opts = Array.from(el.options || []);
			const match = opts.find(o => o.value === value)
				|| opts.find(o => ((o.label || o.text || '').trim() === value));
			if (!match) return 'no <option> matches "' + value + '"';
			el.value = match.value;
			el.dispatchEvent(new Event('input', { bubbles: true }));
			el.dispatchEvent(new Event('change', { bubbles: true }));
			return 'ok';
		}
	"#
    .to_string();
    (script, args)
}

/// Builds a JS function to set an element's value and dispatch input/change events.
fn build_set_value_script(ep: &ElementParams, value: &str) -> (String, Vec<Value>) {
    let args = vec![
        json!({ "type": "string", "value": ep.scope }),
        json!({ "type": "string", "value": ep.selector }),
        json!({ "type": "number", "value": ep.index }),
        json!({ "type": "boolean", "value": ep.has_index }),
        json!({ "type": "string", "value": value }),
    ];
    let script = r#"
		(scope, selector, index, hasIndex, value) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return 'element not found';
			let el;
			if (hasIndex) {
				const all = root.querySelectorAll(selector);
				el = all[index];
			} else {
				el = root.querySelector(selector);
			}
			if (!el) return 'element not found';
			el.focus();
			// Pick the native value setter for the element's ACTUAL type. Calling
			// the HTMLInputElement setter on a <textarea> throws "Illegal
			// invocation" because the setter validates its receiver (issue #117).
			const proto = (el instanceof window.HTMLTextAreaElement)
				? window.HTMLTextAreaElement.prototype
				: (el instanceof window.HTMLInputElement)
					? window.HTMLInputElement.prototype
					: null;
			const nativeSetter = proto
				? Object.getOwnPropertyDescriptor(proto, 'value')?.set
				: null;
			if (nativeSetter) {
				nativeSetter.call(el, value);
			} else {
				el.value = value;
			}
			el.dispatchEvent(new Event('input', { bubbles: true }));
			el.dispatchEvent(new Event('change', { bubbles: true }));
			return 'ok';
		}
	"#
    .to_string();
    (script, args)
}

/// Builds a JS function to focus an element.
fn build_focus_script(ep: &ElementParams) -> (String, Vec<Value>) {
    let args = vec![
        json!({ "type": "string", "value": ep.scope }),
        json!({ "type": "string", "value": ep.selector }),
        json!({ "type": "number", "value": ep.index }),
        json!({ "type": "boolean", "value": ep.has_index }),
    ];
    let script = r#"
		(scope, selector, index, hasIndex) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return 'not found';
			let el;
			if (hasIndex) {
				const all = root.querySelectorAll(selector);
				el = all[index];
			} else {
				el = root.querySelector(selector);
			}
			if (!el) return 'not found';
			el.focus();
			return 'ok';
		}
	"#
    .to_string();
    (script, args)
}

/// Builds a JS function to dispatch an event on an element.
fn build_dispatch_event_script(ep: &ElementParams, event_type: &str, init_json: &str) -> (String, Vec<Value>) {
    let args = vec![
        json!({ "type": "string", "value": ep.scope }),
        json!({ "type": "string", "value": ep.selector }),
        json!({ "type": "number", "value": ep.index }),
        json!({ "type": "boolean", "value": ep.has_index }),
        json!({ "type": "string", "value": event_type }),
        json!({ "type": "string", "value": init_json }),
    ];
    let script = r#"
		(scope, selector, index, hasIndex, eventType, initJSON) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return 'not found';
			let el;
			if (hasIndex) {
				const all = root.querySelectorAll(selector);
				el = all[index];
			} else {
				el = root.querySelector(selector);
			}
			if (!el) return 'not found';
			const init = JSON.parse(initJSON);
			el.dispatchEvent(new Event(eventType, init));
			return 'ok';
		}
	"#
    .to_string();
    (script, args)
}
