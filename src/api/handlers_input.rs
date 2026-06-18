//! Phase 3 input cluster: the page-level keyboard / mouse / wheel / scroll /
//! touch routes. Keyboard press/type reuse the interaction primitives
//! (press_key / type_text); the rest build input.performActions inline.

use std::sync::Arc;

use anyhow::anyhow;
use serde_json::{json, Value};

use super::handlers_interaction::{press_key, type_text};
use super::helpers::{resolve_element, ElementParams};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::new_api_session;
use crate::bidi;

impl Router {
    /// Handles `browserlane:keyboard.press` — presses and releases a key (supports combos).
    pub(crate) async fn handle_keyboard_press(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let key = cmd.params.get("key").and_then(Value::as_str).unwrap_or("").to_string();
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = press_key(&s, &context, &key).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "pressed": true }));
    }

    /// Handles `browserlane:keyboard.down` — presses a key down (no release).
    pub(crate) async fn handle_keyboard_down(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let key = cmd.params.get("key").and_then(Value::as_str).unwrap_or("");
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let resolved = bidi::resolve_key(key);
        let params = json!({
            "context": context,
            "actions": [{
                "type": "key",
                "id": "keyboard",
                "actions": [{ "type": "keyDown", "value": resolved }],
            }],
        });
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "pressed": true }));
    }

    /// Handles `browserlane:keyboard.up` — releases a key.
    pub(crate) async fn handle_keyboard_up(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let key = cmd.params.get("key").and_then(Value::as_str).unwrap_or("");
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let resolved = bidi::resolve_key(key);
        let params = json!({
            "context": context,
            "actions": [{
                "type": "key",
                "id": "keyboard",
                "actions": [{ "type": "keyUp", "value": resolved }],
            }],
        });
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "released": true }));
    }

    /// Handles `browserlane:keyboard.type` — types a string of text.
    pub(crate) async fn handle_keyboard_type(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let text = cmd.params.get("text").and_then(Value::as_str).unwrap_or("").to_string();
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = type_text(&s, &context, &text).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "typed": true }));
    }

    /// Handles `browserlane:mouse.click` — clicks at (x, y) coordinates.
    pub(crate) async fn handle_mouse_click(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let x = cmd.params.get("x").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let y = cmd.params.get("y").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
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
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "clicked": true }));
    }

    /// Handles `browserlane:mouse.move` — moves the mouse to (x, y).
    pub(crate) async fn handle_mouse_move(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let x = cmd.params.get("x").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let y = cmd.params.get("y").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let params = json!({
            "context": context,
            "actions": [{
                "type": "pointer",
                "id": "mouse",
                "parameters": { "pointerType": "mouse" },
                "actions": [{"type": "pointerMove", "x": x, "y": y, "duration": 0}],
            }],
        });
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "moved": true }));
    }

    /// Handles `browserlane:mouse.down` — presses the mouse button down.
    pub(crate) async fn handle_mouse_down(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let button = cmd.params.get("button").and_then(Value::as_f64).map(|b| b as i64).unwrap_or(0);
        let params = json!({
            "context": context,
            "actions": [{
                "type": "pointer",
                "id": "mouse",
                "parameters": { "pointerType": "mouse" },
                "actions": [{"type": "pointerDown", "button": button}],
            }],
        });
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "pressed": true }));
    }

    /// Handles `browserlane:mouse.up` — releases the mouse button.
    pub(crate) async fn handle_mouse_up(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let button = cmd.params.get("button").and_then(Value::as_f64).map(|b| b as i64).unwrap_or(0);
        let params = json!({
            "context": context,
            "actions": [{
                "type": "pointer",
                "id": "mouse",
                "parameters": { "pointerType": "mouse" },
                "actions": [{"type": "pointerUp", "button": button}],
            }],
        });
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "released": true }));
    }

    /// Handles `browserlane:mouse.wheel` — scrolls with deltaX/deltaY at (x, y).
    pub(crate) async fn handle_mouse_wheel(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let x = cmd.params.get("x").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let y = cmd.params.get("y").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let delta_x = cmd.params.get("deltaX").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let delta_y = cmd.params.get("deltaY").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let params = json!({
            "context": context,
            "actions": [{
                "type": "wheel",
                "id": "wheel",
                "actions": [{"type": "scroll", "x": x, "y": y, "deltaX": delta_x, "deltaY": delta_y}],
            }],
        });
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "scrolled": true }));
    }

    /// Handles `browserlane:page.scroll` — scrolls the page (or at an element) in a direction.
    pub(crate) async fn handle_page_scroll(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let direction = match cmd.params.get("direction").and_then(Value::as_str) {
            Some(d) if !d.is_empty() => d.to_string(),
            _ => "down".to_string(),
        };
        let amount = cmd.params.get("amount").and_then(Value::as_f64).map(|a| a as i64).unwrap_or(3);

        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        // Determine scroll target coordinates.
        let (mut x, mut y) = (0i64, 0i64);
        if let Some(selector) = cmd.params.get("selector").and_then(Value::as_str) {
            if !selector.is_empty() {
                let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
                let ep = ElementParams { selector: selector.to_string(), ..Default::default() };
                match resolve_element(&s, &context, ep).await {
                    Ok(info) => {
                        x = (info.box_.x + info.box_.width / 2.0) as i64;
                        y = (info.box_.y + info.box_.height / 2.0) as i64;
                    }
                    Err(e) => return self.send_error(session, cmd.id, &e),
                }
            }
        }

        // Map direction to deltas (120 pixels per scroll "notch").
        let (mut delta_x, mut delta_y) = (0i64, 0i64);
        let pixels = amount * 120;
        match direction.as_str() {
            "down" => delta_y = pixels,
            "up" => delta_y = -pixels,
            "right" => delta_x = pixels,
            "left" => delta_x = -pixels,
            _ => {
                return self.send_error(
                    session,
                    cmd.id,
                    &anyhow!("invalid direction: {direction:?} (use up, down, left, right)"),
                )
            }
        }

        let params = json!({
            "context": context,
            "actions": [{
                "type": "wheel",
                "id": "wheel",
                "actions": [{"type": "scroll", "x": x, "y": y, "deltaX": delta_x, "deltaY": delta_y}],
            }],
        });
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "scrolled": true }));
    }

    /// Handles `browserlane:touch.tap` — touch tap at (x, y).
    pub(crate) async fn handle_touch_tap(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let x = cmd.params.get("x").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let y = cmd.params.get("y").and_then(Value::as_f64).unwrap_or(0.0) as i64;
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
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
        if let Err(e) = self.send_internal_command(session, "input.performActions", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "tapped": true }));
    }
}
