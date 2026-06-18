//! Phase 3 state cluster: the read-state element handlers and routes
//! (text / innerText / html / value / attr / isVisible / isHidden / isEnabled /
//! isChecked / isEditable) plus the exported standalone state-query functions
//! used by the MCP agent. The remaining handlers in this file (highlight,
//! bounds, screenshot, waitFor*, page eval/addScript/addStyle/expose) belong to
//! later clusters and stay unported for now — this file grows incrementally.

use std::sync::Arc;

use std::time::{Duration, Instant};

use anyhow::anyhow;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::sleep;

use super::handlers_elements::semantic_matches_helper;
use super::helpers::{
    build_action_find_script, build_el_base_args, build_el_semantic_args, call_script,
    check_bidi_error, eval_simple_script, extract_element_params, has_semantic, parse_script_result,
    resolve_element, ElementInfo, ElementParams,
};
use super::router::{BidiCommand, BrowserSession, Router, DEFAULT_TIMEOUT};
use super::session::{new_api_session, Session};
use crate::errors::format_go_duration;

// ---------------------------------------------------------------------------
// Router routes — read-state element handlers.
// ---------------------------------------------------------------------------

impl Router {
    /// Handles `browserlane:element.text` — returns element.innerText (trimmed).
    pub(crate) async fn handle_browserlane_el_text(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_state_script(&ep, "(el.innerText || '').trim()");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_element_script(&s, &context, &script, args).await {
            Ok(val) => self.send_success(session, cmd.id, json!({ "text": val })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.innerText` — returns element.innerText (trimmed).
    pub(crate) async fn handle_browserlane_el_inner_text(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_state_script(&ep, "(el.innerText || '').trim()");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_element_script(&s, &context, &script, args).await {
            Ok(val) => self.send_success(session, cmd.id, json!({ "text": val })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.html` — returns element.innerHTML.
    pub(crate) async fn handle_browserlane_el_html(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_state_script(&ep, "el.innerHTML");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_element_script(&s, &context, &script, args).await {
            Ok(val) => self.send_success(session, cmd.id, json!({ "html": val })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.value` — returns element.value (for inputs).
    pub(crate) async fn handle_browserlane_el_value(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_state_script(&ep, "el.value || ''");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_element_script(&s, &context, &script, args).await {
            Ok(val) => self.send_success(session, cmd.id, json!({ "value": val })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.attr` — returns element.getAttribute(name).
    pub(crate) async fn handle_browserlane_el_attr(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let name = cmd.params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = if has_semantic(&ep) {
            let mut args = build_el_semantic_args(&ep);
            args.push(json!({ "type": "string", "value": name }));
            let script = format!(
                r#"
			(scope, selector, role, text, label, placeholder, alt, title, testid, xpath, index, hasIndex, name) => {{
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return JSON.stringify({{error: 'root not found'}});
		{helper}
				const found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
				let el;
				if (hasIndex) {{
					el = found[index];
				}} else {{
					el = pickBest(found, text);
				}}
				if (!el) return JSON.stringify({{error: 'element not found'}});
				const v = el.getAttribute(name);
				return JSON.stringify({{value: v}});
			}}
		"#,
                helper = semantic_matches_helper()
            );
            (script, args)
        } else {
            let mut args = build_el_base_args(&ep);
            args.push(json!({ "type": "string", "value": name }));
            let script = r#"
			(scope, selector, index, hasIndex, name) => {
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return JSON.stringify({error: 'root not found'});
				let el;
				if (hasIndex) {
					el = root.querySelectorAll(selector)[index];
				} else {
					el = root.querySelector(selector);
				}
				if (!el) return JSON.stringify({error: 'element not found'});
				const v = el.getAttribute(name);
				return JSON.stringify({value: v});
			}
		"#
            .to_string();
            (script, args)
        };

        let resp = match self
            .send_internal_command(
                session,
                "script.callFunction",
                json!({
                    "functionDeclaration": script,
                    "target": { "context": context },
                    "arguments": args,
                    "awaitPromise": false,
                    "resultOwnership": "root",
                }),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let val = match parse_script_result(&resp) {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("attr failed: {e}")),
        };

        let result: AttrResult = match serde_json::from_str(&val) {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("attr parse failed: {e}")),
        };
        if !result.error.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("attr: {}", result.error));
        }
        self.send_success(session, cmd.id, json!({ "value": result.value }));
    }

    /// Handles `browserlane:element.isVisible` — checks computed visibility.
    pub(crate) async fn handle_browserlane_el_is_visible(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_bool_script(
            &ep,
            r#"
		const style = window.getComputedStyle(el);
		if (style.display === 'none') return false;
		if (style.visibility === 'hidden') return false;
		if (parseFloat(style.opacity) === 0) return false;
		const rect = el.getBoundingClientRect();
		return rect.width > 0 && rect.height > 0;
	"#,
        );
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_bool_script(&s, &context, &script, args).await {
            Ok(visible) => self.send_success(session, cmd.id, json!({ "visible": visible })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.isHidden` — inverse of isVisible.
    pub(crate) async fn handle_browserlane_el_is_hidden(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_bool_script(
            &ep,
            r#"
		const style = window.getComputedStyle(el);
		if (style.display === 'none') return true;
		if (style.visibility === 'hidden') return true;
		if (parseFloat(style.opacity) === 0) return true;
		const rect = el.getBoundingClientRect();
		return rect.width === 0 || rect.height === 0;
	"#,
        );
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_bool_script(&s, &context, &script, args).await {
            Ok(hidden) => self.send_success(session, cmd.id, json!({ "hidden": hidden })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.isEnabled` — checks !element.disabled.
    pub(crate) async fn handle_browserlane_el_is_enabled(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_bool_script(&ep, "return !el.disabled;");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_bool_script(&s, &context, &script, args).await {
            Ok(enabled) => self.send_success(session, cmd.id, json!({ "enabled": enabled })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.isChecked` — returns element.checked.
    pub(crate) async fn handle_browserlane_el_is_checked(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_bool_script(&ep, "return !!el.checked;");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_bool_script(&s, &context, &script, args).await {
            Ok(checked) => self.send_success(session, cmd.id, json!({ "checked": checked })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:element.isEditable` — not disabled and not readonly.
    pub(crate) async fn handle_browserlane_el_is_editable(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_bool_script(&ep, "return !el.disabled && !el.readOnly;");
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_bool_script(&s, &context, &script, args).await {
            Ok(editable) => self.send_success(session, cmd.id, json!({ "editable": editable })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }
}

/// The `{value, error}` payload returned by the attr scripts.
#[derive(Debug, Default, Deserialize)]
struct AttrResult {
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    error: String,
}

// ---------------------------------------------------------------------------
// Script builder helpers for state queries.
// ---------------------------------------------------------------------------

/// Builds a script that finds an element and evaluates a string expression.
/// The expression receives `el` and should return a string.
pub(crate) fn build_el_state_script(ep: &ElementParams, expr: &str) -> (String, Vec<Value>) {
    if has_semantic(ep) {
        let args = build_el_semantic_args(ep);
        let script = format!(
            r#"
			(scope, selector, role, text, label, placeholder, alt, title, testid, xpath, index, hasIndex) => {{
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return null;
		{helper}
				const found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
				let el;
				if (hasIndex) {{
					el = found[index];
				}} else {{
					el = pickBest(found, text);
				}}
				if (!el) return null;
				return {expr};
			}}
		"#,
            helper = semantic_matches_helper()
        );
        return (script, args);
    }

    let args = build_el_base_args(ep);
    let script = format!(
        r#"
		(scope, selector, index, hasIndex) => {{
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return null;
			let el;
			if (hasIndex) {{
				el = root.querySelectorAll(selector)[index];
			}} else {{
				el = root.querySelector(selector);
			}}
			if (!el) return null;
			return {expr};
		}}
	"#
    );
    (script, args)
}

/// Builds a script that finds an element and evaluates a boolean expression.
/// The body receives `el` and should use `return true/false;`.
fn build_el_bool_script(ep: &ElementParams, body: &str) -> (String, Vec<Value>) {
    if has_semantic(ep) {
        let args = build_el_semantic_args(ep);
        let script = format!(
            r#"
			(scope, selector, role, text, label, placeholder, alt, title, testid, xpath, index, hasIndex) => {{
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return 'error:root not found';
		{helper}
				const found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
				let el;
				if (hasIndex) {{
					el = found[index];
				}} else {{
					el = pickBest(found, text);
				}}
				if (!el) return 'error:element not found';
				const _check = (el) => {{ {body} }};
				return _check(el) ? 'true' : 'false';
			}}
		"#,
            helper = semantic_matches_helper()
        );
        return (script, args);
    }

    let args = build_el_base_args(ep);
    let script = format!(
        r#"
		(scope, selector, index, hasIndex) => {{
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return 'error:root not found';
			let el;
			if (hasIndex) {{
				el = root.querySelectorAll(selector)[index];
			}} else {{
				el = root.querySelector(selector);
			}}
			if (!el) return 'error:element not found';
			const _check = (el) => {{ {body} }};
			return _check(el) ? 'true' : 'false';
		}}
	"#
    );
    (script, args)
}

// ---------------------------------------------------------------------------
// Exported standalone state helpers — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Runs a state script via the Session and returns the string result.
pub async fn eval_element_script(
    s: &dyn Session,
    context: &str,
    script: &str,
    args: Vec<Value>,
) -> anyhow::Result<String> {
    let resp = call_script(s, context, script, args).await?;
    parse_script_result(&resp).map_err(|_| anyhow!("element not found"))
}

/// Runs a boolean script via the Session and parses the "true"/"false" result.
pub async fn eval_bool_script(
    s: &dyn Session,
    context: &str,
    script: &str,
    args: Vec<Value>,
) -> anyhow::Result<bool> {
    let resp = call_script(s, context, script, args).await?;
    let val = parse_script_result(&resp).map_err(|_| anyhow!("element not found"))?;

    if val.len() > 6 && &val[..6] == "error:" {
        return Err(anyhow!("{}", &val[6..]));
    }
    Ok(val == "true")
}

/// Returns the visible text of an element (innerText).
pub async fn get_text(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<String> {
    let (script, args) = build_el_state_script(&ep, "(el.innerText || '').trim()");
    eval_element_script(s, context, &script, args).await
}

/// Returns the innerText of an element.
pub async fn get_inner_text(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<String> {
    let (script, args) = build_el_state_script(&ep, "(el.innerText || '').trim()");
    eval_element_script(s, context, &script, args).await
}

/// Returns the innerHTML of an element.
pub async fn get_inner_html(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<String> {
    let (script, args) = build_el_state_script(&ep, "el.innerHTML");
    eval_element_script(s, context, &script, args).await
}

/// Returns the outerHTML of an element.
pub async fn get_outer_html(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<String> {
    let (script, args) = build_el_state_script(&ep, "el.outerHTML");
    eval_element_script(s, context, &script, args).await
}

/// Returns the value property of a form element.
pub async fn get_value(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<String> {
    let (script, args) = build_el_state_script(&ep, "el.value || ''");
    eval_element_script(s, context, &script, args).await
}

/// Returns the value of an HTML attribute on an element.
pub async fn get_attribute(
    s: &dyn Session,
    context: &str,
    ep: ElementParams,
    name: &str,
) -> anyhow::Result<String> {
    let (script, args) = if has_semantic(&ep) {
        let mut args = build_el_semantic_args(&ep);
        args.push(json!({ "type": "string", "value": name }));
        let script = format!(
            r#"
			(scope, selector, role, text, label, placeholder, alt, title, testid, xpath, index, hasIndex, name) => {{
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return null;
		{helper}
				const found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
				let el;
				if (hasIndex) {{
					el = found[index];
				}} else {{
					el = pickBest(found, text);
				}}
				if (!el) return null;
				const v = el.getAttribute(name);
				return v === null ? '' : v;
			}}
		"#,
            helper = semantic_matches_helper()
        );
        (script, args)
    } else {
        let mut args = build_el_base_args(&ep);
        args.push(json!({ "type": "string", "value": name }));
        let script = r#"
			(scope, selector, index, hasIndex, name) => {
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return null;
				let el;
				if (hasIndex) {
					el = root.querySelectorAll(selector)[index];
				} else {
					el = root.querySelector(selector);
				}
				if (!el) return null;
				const v = el.getAttribute(name);
				return v === null ? '' : v;
			}
		"#
        .to_string();
        (script, args)
    };

    eval_element_script(s, context, &script, args).await
}

/// Checks if an element is visible (not hidden, not zero-size).
pub async fn is_visible(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<bool> {
    let (script, args) = build_el_bool_script(
        &ep,
        r#"
		const style = window.getComputedStyle(el);
		if (style.display === 'none') return false;
		if (style.visibility === 'hidden') return false;
		if (parseFloat(style.opacity) === 0) return false;
		const rect = el.getBoundingClientRect();
		return rect.width > 0 && rect.height > 0;
	"#,
    );
    eval_bool_script(s, context, &script, args).await
}

/// Checks if an element is enabled (!disabled).
pub async fn is_enabled(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<bool> {
    let (script, args) = build_el_bool_script(&ep, "return !el.disabled;");
    eval_bool_script(s, context, &script, args).await
}

/// Counts elements matching a CSS selector.
pub async fn get_count(s: &dyn Session, context: &str, selector: &str) -> anyhow::Result<i64> {
    // Quote the selector as a JS string literal (Go uses %q).
    let quoted = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{selector}\""));
    let expr = format!("() => String(document.querySelectorAll({quoted}).length)");
    let val = eval_simple_script(s, context, &expr).await?;
    val.trim()
        .parse::<i64>()
        .map_err(|e| anyhow!("failed to parse count: {e}"))
}

// ---------------------------------------------------------------------------
// Page-level evaluation handlers (eval cluster).
// ---------------------------------------------------------------------------

impl Router {
    /// Handles `browserlane:page.eval` — evaluates a JS expression and returns the result.
    pub(crate) async fn handle_page_eval(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let expression = cmd.params.get("expression").and_then(Value::as_str).unwrap_or("").to_string();
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let resp = match self
            .send_internal_command(
                session,
                "script.evaluate",
                json!({
                    "expression": expression,
                    "target": { "context": context },
                    "awaitPromise": true,
                    "resultOwnership": "none",
                }),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }

        let value = match deserialize_script_result(&resp) {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("eval failed: {e}")),
        };
        self.send_success(session, cmd.id, json!({ "value": value }));
    }

    /// Handles `browserlane:page.addScript` — injects a `<script>` tag (url or content).
    pub(crate) async fn handle_page_add_script(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let url = cmd.params.get("url").and_then(Value::as_str).unwrap_or("");
        let content = cmd.params.get("content").and_then(Value::as_str).unwrap_or("");
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let script = if !url.is_empty() {
            r#"(url) => {
			return new Promise((resolve, reject) => {
				const s = document.createElement('script');
				s.src = url;
				s.onload = () => resolve('ok');
				s.onerror = () => reject(new Error('failed to load script'));
				document.head.appendChild(s);
			});
		}"#
        } else {
            r#"(content) => {
			const s = document.createElement('script');
			s.textContent = content;
			document.head.appendChild(s);
			return 'ok';
		}"#
        };

        let arg = if url.is_empty() { content } else { url };
        let params = json!({
            "functionDeclaration": script,
            "target": { "context": context },
            "arguments": [{ "type": "string", "value": arg }],
            "awaitPromise": true,
            "resultOwnership": "root",
        });
        if let Err(e) = self.send_internal_command(session, "script.callFunction", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "added": true }));
    }

    /// Handles `browserlane:page.addStyle` — injects a `<style>`/`<link>` tag (url or content).
    pub(crate) async fn handle_page_add_style(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let url = cmd.params.get("url").and_then(Value::as_str).unwrap_or("");
        let content = cmd.params.get("content").and_then(Value::as_str).unwrap_or("");
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let script = if !url.is_empty() {
            r#"(url) => {
			return new Promise((resolve, reject) => {
				const link = document.createElement('link');
				link.rel = 'stylesheet';
				link.href = url;
				link.onload = () => resolve('ok');
				link.onerror = () => reject(new Error('failed to load stylesheet'));
				document.head.appendChild(link);
			});
		}"#
        } else {
            r#"(content) => {
			const s = document.createElement('style');
			s.textContent = content;
			document.head.appendChild(s);
			return 'ok';
		}"#
        };

        let arg = if url.is_empty() { content } else { url };
        let params = json!({
            "functionDeclaration": script,
            "target": { "context": context },
            "arguments": [{ "type": "string", "value": arg }],
            "awaitPromise": true,
            "resultOwnership": "root",
        });
        if let Err(e) = self.send_internal_command(session, "script.callFunction", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "added": true }));
    }

    /// Handles `browserlane:element.screenshot` — captures a clipped element screenshot.
    pub(crate) async fn handle_browserlane_el_screenshot(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        // Resolve element to get its bounding box (also scrolls it into view).
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let info = match resolve_element(&s, &context, ep).await {
            Ok(i) => i,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let clip_params = json!({
            "context": context,
            "clip": {
                "type": "box",
                "x": info.box_.x,
                "y": info.box_.y,
                "width": info.box_.width,
                "height": info.box_.height,
            },
        });

        let resp = match self.send_internal_command(session, "browsingContext.captureScreenshot", clip_params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }

        let data = resp.get("result").and_then(|r| r.get("data")).and_then(Value::as_str).unwrap_or("");
        self.send_success(session, cmd.id, json!({ "data": data }));
    }

    /// Handles `browserlane:page.expose` — injects a named function on `window` via JS.
    pub(crate) async fn handle_page_expose(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let name = cmd.params.get("name").and_then(Value::as_str).unwrap_or("");
        let fn_ = cmd.params.get("fn").and_then(Value::as_str).unwrap_or("");
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let script = r#"(name, fn) => {
		window[name] = new Function('return ' + fn)();
		return 'ok';
	}"#;
        let params = json!({
            "functionDeclaration": script,
            "target": { "context": context },
            "arguments": [
                { "type": "string", "value": name },
                { "type": "string", "value": fn_ },
            ],
            "awaitPromise": false,
            "resultOwnership": "root",
        });
        if let Err(e) = self.send_internal_command(session, "script.callFunction", params).await {
            return self.send_error(session, cmd.id, &e);
        }
        self.send_success(session, cmd.id, json!({ "exposed": true }));
    }
}

/// Extracts a usable value from a BiDi script result. Handles primitives
/// (string / number / boolean / null / undefined) and objects / arrays.
fn deserialize_script_result(resp: &Value) -> anyhow::Result<Value> {
    let inner = resp.get("result").and_then(|r| r.get("result"));
    let typ = inner.and_then(|r| r.get("type")).and_then(Value::as_str).unwrap_or("");
    let value = inner.and_then(|r| r.get("value")).cloned().unwrap_or(Value::Null);

    match typ {
        "null" | "undefined" => Ok(Value::Null),
        "string" | "number" | "boolean" => Ok(value),
        "array" => {
            // BiDi arrays: {type: "array", value: [{type, value}, ...]}.
            if let Value::Array(items) = &value {
                let out: Vec<Value> = items
                    .iter()
                    .map(|item| item.get("value").cloned().unwrap_or_else(|| item.clone()))
                    .collect();
                Ok(Value::Array(out))
            } else {
                Ok(value)
            }
        }
        "object" => {
            // BiDi objects: {type: "object", value: [[key, {type, value}], ...]}.
            if let Value::Array(pairs) = &value {
                let mut out = serde_json::Map::new();
                for pair in pairs {
                    if let Value::Array(kv) = pair {
                        if kv.len() == 2 {
                            if let Some(key) = kv[0].as_str() {
                                let v = kv[1].get("value").cloned().unwrap_or(Value::Null);
                                out.insert(key.to_string(), v);
                            }
                        }
                    }
                }
                Ok(Value::Object(out))
            } else {
                Ok(value)
            }
        }
        _ => Ok(value),
    }
}

// ---------------------------------------------------------------------------
// State-extras cluster: bounds / highlight / wait family routes.
// ---------------------------------------------------------------------------

impl Router {
    /// Handles `browserlane:element.bounds` — returns getBoundingClientRect().
    pub(crate) async fn handle_browserlane_el_bounds(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_json_script(
            &ep,
            r#"
		const rect = el.getBoundingClientRect();
		return JSON.stringify({x: rect.x, y: rect.y, width: rect.width, height: rect.height});
	"#,
        );

        let resp = match self
            .send_internal_command(
                session,
                "script.callFunction",
                json!({
                    "functionDeclaration": script,
                    "target": { "context": context },
                    "arguments": args,
                    "awaitPromise": false,
                    "resultOwnership": "root",
                }),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let val = match parse_script_result(&resp) {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("bounds failed: {e}")),
        };

        match serde_json::from_str::<super::helpers::BoxInfo>(&val) {
            Ok(box_) => self.send_success(
                session,
                cmd.id,
                json!({ "x": box_.x, "y": box_.y, "width": box_.width, "height": box_.height }),
            ),
            Err(e) => self.send_error(session, cmd.id, &anyhow!("bounds parse failed: {e}")),
        }
    }

    /// Handles `browserlane:element.highlight` — briefly outlines an element.
    pub(crate) async fn handle_browserlane_el_highlight(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_state_script(
            &ep,
            r#"(() => {
		const prevOutline = el.style.outline;
		const prevOffset = el.style.outlineOffset;
		el.style.outline = '2px solid #ff2d95';
		el.style.outlineOffset = '1px';
		setTimeout(() => { el.style.outline = prevOutline; el.style.outlineOffset = prevOffset; }, 2000);
		return 'ok';
	})()"#,
        );
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let val = match eval_element_script(&s, &context, &script, args).await {
            Ok(v) => v,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if val != "ok" {
            return self.send_error(session, cmd.id, &anyhow!("highlight: {val}"));
        }
        self.send_success(session, cmd.id, json!({ "highlighted": true }));
    }

    /// Handles `browserlane:element.waitFor` — waits for element state
    /// (visible / hidden / attached / detached).
    pub(crate) async fn handle_browserlane_el_wait_for(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let state = match cmd.params.get("state").and_then(Value::as_str) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => "visible".to_string(),
        };
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);

        let deadline = Instant::now() + ep.timeout;
        let interval = Duration::from_millis(100);

        loop {
            let mut met = false;
            let mut check_err = false;

            match state.as_str() {
                "attached" => {
                    met = resolve_element_no_wait(&s, &context, ep.clone()).await.is_ok();
                }
                "detached" => {
                    met = resolve_element_no_wait(&s, &context, ep.clone()).await.is_err();
                }
                "visible" => {
                    if resolve_element_no_wait(&s, &context, ep.clone()).await.is_ok() {
                        let (script, args) = build_el_bool_script(
                            &ep,
                            r#"
					const style = window.getComputedStyle(el);
					if (style.display === 'none') return false;
					if (style.visibility === 'hidden') return false;
					if (parseFloat(style.opacity) === 0) return false;
					const rect = el.getBoundingClientRect();
					return rect.width > 0 && rect.height > 0;
				"#,
                        );
                        match eval_bool_script(&s, &context, &script, args).await {
                            Ok(v) => met = v,
                            Err(_) => check_err = true,
                        }
                    }
                }
                "hidden" => {
                    if resolve_element_no_wait(&s, &context, ep.clone()).await.is_err() {
                        met = true;
                    } else {
                        let (script, args) = build_el_bool_script(
                            &ep,
                            r#"
					const style = window.getComputedStyle(el);
					if (style.display === 'none') return true;
					if (style.visibility === 'hidden') return true;
					if (parseFloat(style.opacity) === 0) return true;
					const rect = el.getBoundingClientRect();
					return rect.width === 0 || rect.height === 0;
				"#,
                        );
                        match eval_bool_script(&s, &context, &script, args).await {
                            Ok(v) => met = v,
                            Err(_) => check_err = true,
                        }
                    }
                }
                _ => {
                    return self.send_error(
                        session,
                        cmd.id,
                        &anyhow!("unknown state: {state} (expected visible, hidden, attached, detached)"),
                    )
                }
            }

            if !check_err && met {
                return self.send_success(session, cmd.id, json!({ "state": state }));
            }

            if Instant::now() > deadline {
                return self.send_error(session, cmd.id, &anyhow!("timeout waiting for element to be {state}"));
            }
            sleep(interval).await;
        }
    }

    /// Handles `browserlane:page.waitFor` — waits for a selector to appear.
    pub(crate) async fn handle_page_wait_for(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match resolve_element(&s, &context, ep).await {
            Ok(info) => self.send_success(
                session,
                cmd.id,
                json!({
                    "tag": info.tag,
                    "text": info.text,
                    "box": { "x": info.box_.x, "y": info.box_.y, "width": info.box_.width, "height": info.box_.height },
                }),
            ),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Handles `browserlane:page.wait` — client-side delay.
    pub(crate) async fn handle_page_wait(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let ms = cmd.params.get("ms").and_then(Value::as_f64).unwrap_or(0.0);
        if ms <= 0.0 {
            return self.send_success(session, cmd.id, json!({ "waited": true }));
        }
        sleep(Duration::from_millis(ms as u64)).await;
        self.send_success(session, cmd.id, json!({ "waited": true }));
    }

    /// Handles `browserlane:page.waitForFunction` — polls until a JS predicate is truthy.
    pub(crate) async fn handle_page_wait_for_function(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let fn_ = cmd.params.get("fn").and_then(Value::as_str).unwrap_or("");
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let timeout = match cmd.params.get("timeout").and_then(Value::as_f64) {
            Some(ms) if ms > 0.0 => Duration::from_millis(ms as u64),
            _ => DEFAULT_TIMEOUT,
        };

        // Callers pass either a full function or a bare boolean expression. Wrap
        // uniformly: evaluate the operand and invoke it if it is a function.
        let wrapped = format!(
            "() => {{ const __browserlanePred = ({}); return (typeof __browserlanePred === 'function') ? __browserlanePred() : __browserlanePred; }}",
            fn_.trim().trim_end_matches(';')
        );

        let deadline = Instant::now() + timeout;
        let interval = Duration::from_millis(100);
        let mut last_err = String::new();

        loop {
            let resp = self
                .send_internal_command(
                    session,
                    "script.callFunction",
                    json!({
                        "functionDeclaration": wrapped,
                        "target": { "context": context },
                        "arguments": [],
                        "awaitPromise": true,
                        "resultOwnership": "root",
                    }),
                )
                .await;

            if let Ok(resp) = resp {
                let r = resp.get("result");
                let outer_type = r.and_then(|r| r.get("type")).and_then(Value::as_str).unwrap_or("");
                if outer_type == "exception" {
                    last_err = r
                        .and_then(|r| r.get("exceptionDetails"))
                        .and_then(|e| e.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                } else {
                    let inner = r.and_then(|r| r.get("result"));
                    let typ = inner.and_then(|r| r.get("type")).and_then(Value::as_str).unwrap_or("");
                    let value = inner.and_then(|r| r.get("value")).cloned().unwrap_or(Value::Null);
                    let truthy = match typ {
                        "boolean" => value.as_bool() == Some(true),
                        "number" => value.as_f64().map(|v| v != 0.0).unwrap_or(false),
                        "string" => value.as_str().map(|v| !v.is_empty()).unwrap_or(false),
                        "null" | "undefined" => false,
                        _ => !value.is_null(),
                    };
                    if truthy {
                        return self.send_success(session, cmd.id, json!({ "value": value }));
                    }
                }
            }

            if Instant::now() > deadline {
                if !last_err.is_empty() {
                    return self.send_error(
                        session,
                        cmd.id,
                        &anyhow!("timeout waiting for function to return truthy (last error: {last_err})"),
                    );
                }
                return self.send_error(session, cmd.id, &anyhow!("timeout waiting for function to return truthy"));
            }
            sleep(interval).await;
        }
    }
}

/// Builds a script that finds an element and returns JSON.
/// The body receives `el` and should use `return JSON.stringify(...)`.
fn build_el_json_script(ep: &ElementParams, body: &str) -> (String, Vec<Value>) {
    if has_semantic(ep) {
        let args = build_el_semantic_args(ep);
        let script = format!(
            r#"
			(scope, selector, role, text, label, placeholder, alt, title, testid, xpath, index, hasIndex) => {{
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return JSON.stringify({{error: 'root not found'}});
		{helper}
				const found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
				let el;
				if (hasIndex) {{
					el = found[index];
				}} else {{
					el = pickBest(found, text);
				}}
				if (!el) return JSON.stringify({{error: 'element not found'}});
				{body}
			}}
		"#,
            helper = semantic_matches_helper()
        );
        return (script, args);
    }

    let args = build_el_base_args(ep);
    let script = format!(
        r#"
		(scope, selector, index, hasIndex) => {{
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return JSON.stringify({{error: 'root not found'}});
			let el;
			if (hasIndex) {{
				el = root.querySelectorAll(selector)[index];
			}} else {{
				el = root.querySelector(selector);
			}}
			if (!el) return JSON.stringify({{error: 'element not found'}});
			{body}
		}}
	"#
    );
    (script, args)
}

// ---------------------------------------------------------------------------
// Exported standalone wait functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Tries to find an element immediately, without polling.
pub async fn resolve_element_no_wait(
    s: &dyn Session,
    context: &str,
    ep: ElementParams,
) -> anyhow::Result<ElementInfo> {
    let (script, args) = build_action_find_script(&ep);
    let resp = call_script(s, context, &script, args).await?;

    let inner = resp.get("result").and_then(|r| r.get("result"));
    let typ = inner.and_then(|r| r.get("type")).and_then(Value::as_str).unwrap_or("");
    let value = inner.and_then(|r| r.get("value")).and_then(Value::as_str).unwrap_or("");
    if typ != "string" || value.is_empty() {
        return Err(anyhow!("element not found"));
    }
    serde_json::from_str::<ElementInfo>(value).map_err(|e| anyhow!("failed to parse element info: {e}"))
}

/// Polls until the element exists and is visible, or times out.
pub async fn wait_for_visible(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    let ep = ep.with_default_timeout();
    let deadline = Instant::now() + ep.timeout;
    let interval = Duration::from_millis(100);

    loop {
        if resolve_element_no_wait(s, context, ep.clone()).await.is_ok() {
            if let Ok(true) = is_visible(s, context, ep.clone()).await {
                return Ok(());
            }
        }
        if Instant::now() > deadline {
            return Err(anyhow!("timeout after {}: element not visible", format_go_duration(ep.timeout)));
        }
        sleep(interval).await;
    }
}

/// Polls until the element is either not found or not visible.
pub async fn wait_for_hidden(s: &dyn Session, context: &str, ep: ElementParams) -> anyhow::Result<()> {
    let ep = ep.with_default_timeout();
    let deadline = Instant::now() + ep.timeout;
    let interval = Duration::from_millis(100);

    loop {
        if resolve_element_no_wait(s, context, ep.clone()).await.is_err() {
            return Ok(()); // not found = hidden
        }
        match is_visible(s, context, ep.clone()).await {
            Ok(false) | Err(_) => return Ok(()), // not visible = hidden
            _ => {}
        }
        if Instant::now() > deadline {
            return Err(anyhow!("timeout after {}: element still visible", format_go_duration(ep.timeout)));
        }
        sleep(interval).await;
    }
}

/// Waits until the page body contains the given text.
pub async fn wait_for_text(s: &dyn Session, context: &str, text: &str, timeout: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let interval = Duration::from_millis(100);

    loop {
        if let Ok(page_text) = eval_simple_script(s, context, "() => document.body.innerText").await {
            if page_text.contains(text) {
                return Ok(());
            }
        }
        if Instant::now() > deadline {
            return Err(anyhow!("timeout waiting for text {text:?} to appear"));
        }
        sleep(interval).await;
    }
}

/// Waits until a JS expression returns a truthy value.
pub async fn wait_for_function(
    s: &dyn Session,
    context: &str,
    expression: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let deadline = Instant::now() + timeout;
    let interval = Duration::from_millis(100);
    let func = format!("() => {{ const r = {expression}; return r ? String(r) : ''; }}");

    loop {
        if let Ok(val) = eval_simple_script(s, context, &func).await {
            if !val.is_empty() {
                return Ok(val);
            }
        }
        if Instant::now() > deadline {
            return Err(anyhow!("timeout waiting for expression to return truthy: {expression}"));
        }
        sleep(interval).await;
    }
}
