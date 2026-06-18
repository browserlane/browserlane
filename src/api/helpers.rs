//! Phase 2 ports only the navigation-slice subset: BiDi error/result parsing
//! and simple script evaluation. The element-resolution machinery (ResolveElement,
//! buildActionFindScript, semantic selectors, etc.) is ported in Phase 3.

use std::time::Duration;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::sleep;

use super::handlers_elements::{describe_selector, semantic_matches_helper};
use super::router::DEFAULT_TIMEOUT;
use super::session::Session;

// NOTE: ElementInfo/BoxInfo's faithful home is handlers_elements.go; they live
// here in Phase 2 because the Session abstraction references BoxInfo. They move
// to handlers_elements.rs when that file is ported in Phase 3.

/// Element description returned by find scripts.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, rename = "box")]
    pub box_: BoxInfo,
}

/// Bounding box of an element.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoxInfo {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
}

/// Checks if a BiDi response is an error and returns it.
/// BiDi error responses have: `{ "type": "error", "error": "...", "message": "..." }`.
pub fn check_bidi_error(resp: &Value) -> anyhow::Result<()> {
    if resp.get("type").and_then(Value::as_str) == Some("error") {
        let error = resp.get("error").and_then(Value::as_str).unwrap_or("");
        let message = resp.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(anyhow!("{error}: {message}"));
    }
    Ok(())
}

/// Parses a BiDi script.callFunction response and returns the string value.
pub fn parse_script_result(resp: &Value) -> anyhow::Result<String> {
    let result = resp.get("result");

    let outer_type = result
        .and_then(|r| r.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if outer_type == "exception" {
        let text = result
            .and_then(|r| r.get("exceptionDetails"))
            .and_then(|e| e.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let text = if text.is_empty() {
            "script threw an exception"
        } else {
            text
        };
        return Err(anyhow!("{text}"));
    }

    let inner = result.and_then(|r| r.get("result"));
    let inner_type = inner
        .and_then(|r| r.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if inner_type == "null" || inner_type == "undefined" {
        return Err(anyhow!("script returned {inner_type}"));
    }

    let value = inner
        .and_then(|r| r.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("");
    Ok(value.to_string())
}

/// Runs a no-argument script.callFunction via the Session and returns the
/// string result.
pub async fn eval_simple_script(s: &dyn Session, context: &str, func: &str) -> anyhow::Result<String> {
    let params = json!({
        "functionDeclaration": func,
        "target": { "context": context },
        "arguments": [],
        "awaitPromise": false,
        "resultOwnership": "root",
    });

    let resp = s.send_bidi_command("script.callFunction", params).await?;
    parse_script_result(&resp)
}

/// Runs a script.callFunction with arguments via the Session and returns the
/// raw response.
pub async fn call_script(
    s: &dyn Session,
    context: &str,
    func: &str,
    args: Vec<Value>,
) -> anyhow::Result<Value> {
    let params = json!({
        "functionDeclaration": func,
        "target": { "context": context },
        "arguments": args,
        "awaitPromise": false,
        "resultOwnership": "root",
    });

    s.send_bidi_command("script.callFunction", params).await
}

// ---------------------------------------------------------------------------
// Element-resolution kernel (shared by find / state / interaction / input).
// ---------------------------------------------------------------------------

/// Extracted parameters for element resolution.
#[derive(Debug, Default, Clone)]
pub struct ElementParams {
    pub selector: String,
    pub index: i64,
    pub has_index: bool,
    pub scope: String,
    pub role: String,
    pub text: String,
    pub label: String,
    pub placeholder: String,
    pub alt: String,
    pub title: String,
    pub testid: String,
    pub xpath: String,
    pub context: String,
    pub timeout: Duration,
    pub force: bool,
}

impl ElementParams {
    /// Applies the auto-wait default timeout when unset.
    pub fn with_default_timeout(mut self) -> Self {
        if self.timeout.is_zero() {
            self.timeout = DEFAULT_TIMEOUT;
        }
        self
    }
}

/// Extracts element parameters from command params.
pub fn extract_element_params(params: &serde_json::Map<String, Value>) -> ElementParams {
    let s = |k: &str| params.get(k).and_then(Value::as_str).unwrap_or("").to_string();

    let mut ep = ElementParams {
        timeout: DEFAULT_TIMEOUT,
        selector: s("selector"),
        context: s("context"),
        scope: s("scope"),
        role: s("role"),
        text: s("text"),
        label: s("label"),
        placeholder: s("placeholder"),
        alt: s("alt"),
        title: s("title"),
        testid: s("testid"),
        xpath: s("xpath"),
        ..Default::default()
    };

    if let Some(idx) = params.get("index").and_then(Value::as_f64) {
        ep.index = idx as i64;
        ep.has_index = true;
    }
    if let Some(ms) = params.get("timeout").and_then(Value::as_f64) {
        if ms > 0.0 {
            ep.timeout = Duration::from_millis(ms as u64);
        }
    }
    if let Some(force) = params.get("force").and_then(Value::as_bool) {
        ep.force = force;
    }

    ep
}

/// Returns true if any semantic selector params are set.
pub fn has_semantic(ep: &ElementParams) -> bool {
    !ep.role.is_empty()
        || !ep.text.is_empty()
        || !ep.label.is_empty()
        || !ep.placeholder.is_empty()
        || !ep.alt.is_empty()
        || !ep.title.is_empty()
        || !ep.testid.is_empty()
        || !ep.xpath.is_empty()
}

/// Standard [scope, selector, index, hasIndex] args.
pub fn build_el_base_args(ep: &ElementParams) -> Vec<Value> {
    vec![
        json!({ "type": "string", "value": ep.scope }),
        json!({ "type": "string", "value": ep.selector }),
        json!({ "type": "number", "value": ep.index }),
        json!({ "type": "boolean", "value": ep.has_index }),
    ]
}

/// The 12-arg list for semantic element resolution.
pub fn build_el_semantic_args(ep: &ElementParams) -> Vec<Value> {
    vec![
        json!({ "type": "string", "value": ep.scope }),
        json!({ "type": "string", "value": ep.selector }),
        json!({ "type": "string", "value": ep.role }),
        json!({ "type": "string", "value": ep.text }),
        json!({ "type": "string", "value": ep.label }),
        json!({ "type": "string", "value": ep.placeholder }),
        json!({ "type": "string", "value": ep.alt }),
        json!({ "type": "string", "value": ep.title }),
        json!({ "type": "string", "value": ep.testid }),
        json!({ "type": "string", "value": ep.xpath }),
        json!({ "type": "number", "value": ep.index }),
        json!({ "type": "boolean", "value": ep.has_index }),
    ]
}

/// Builds a JS function that finds an element (CSS or semantic), scrolls it into
/// view, and returns its info as JSON.
pub fn build_action_find_script(ep: &ElementParams) -> (String, Vec<Value>) {
    if !has_semantic(ep) && !ep.selector.is_empty() {
        let args = build_el_base_args(ep);
        let script = r#"
			(scope, selector, index, hasIndex) => {
				const root = scope ? document.querySelector(scope) : document;
				if (!root) return null;
				let el;
				if (hasIndex) {
					const all = root.querySelectorAll(selector);
					el = all[index];
				} else {
					el = root.querySelector(selector);
				}
				if (!el) return null;
				if (el.scrollIntoViewIfNeeded) {
					el.scrollIntoViewIfNeeded(true);
				} else {
					el.scrollIntoView({ block: 'center', inline: 'nearest' });
				}
				const rect = el.getBoundingClientRect();
				return JSON.stringify({
					tag: el.tagName.toLowerCase(),
					text: (el.innerText || '').trim(),
					box: { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
				});
			}
		"#
        .to_string();
        return (script, args);
    }

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
			if (el.scrollIntoViewIfNeeded) {{
				el.scrollIntoViewIfNeeded(true);
			}} else {{
				el.scrollIntoView({{ block: 'center', inline: 'nearest' }});
			}}
			const rect = el.getBoundingClientRect();
			return JSON.stringify(toInfo(el));
		}}
	"#,
        helper = semantic_matches_helper()
    );
    (script, args)
}

/// Builds a JS function that finds an element and returns it directly (BiDi
/// serializes the DOM node with a sharedId).
pub fn build_ref_find_script(ep: &ElementParams) -> (String, Vec<Value>) {
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
				return el || null;
			}}
		"#,
            helper = semantic_matches_helper()
        );
        return (script, args);
    }

    let args = build_el_base_args(ep);
    let script = r#"
		(scope, selector, index, hasIndex) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return null;
			let el;
			if (hasIndex) {
				const all = root.querySelectorAll(selector);
				el = all[index];
			} else {
				el = root.querySelector(selector);
			}
			return el || null;
		}
	"#
    .to_string();
    (script, args)
}

/// Finds an element using the given params, polling until found or timeout.
pub async fn resolve_element(
    s: &dyn Session,
    context: &str,
    ep: ElementParams,
) -> anyhow::Result<ElementInfo> {
    let ep = ep.with_default_timeout();
    let (script, args) = build_action_find_script(&ep);
    let info = wait_for_element_with_script(s, context, &script, args, ep.timeout).await?;
    s.set_last_element_box(info.box_);
    Ok(info)
}

/// Finds an element and returns its BiDi sharedId.
pub async fn resolve_element_ref(
    s: &dyn Session,
    context: &str,
    ep: ElementParams,
) -> anyhow::Result<String> {
    let ep = ep.with_default_timeout();
    let (script, args) = build_ref_find_script(&ep);
    let deadline = std::time::Instant::now() + ep.timeout;

    loop {
        if let Ok(resp) = call_script(s, context, &script, args.clone()).await {
            let inner = resp.get("result").and_then(|r| r.get("result"));
            let typ = inner.and_then(|r| r.get("type")).and_then(Value::as_str).unwrap_or("");
            let shared = inner.and_then(|r| r.get("sharedId")).and_then(Value::as_str).unwrap_or("");
            if typ == "node" && !shared.is_empty() {
                return Ok(shared.to_string());
            }
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("timeout waiting for element: not found"));
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// Polls until an element is found using a custom script.
pub async fn wait_for_element_with_script(
    s: &dyn Session,
    context: &str,
    script: &str,
    args: Vec<Value>,
    timeout: Duration,
) -> anyhow::Result<ElementInfo> {
    let deadline = std::time::Instant::now() + timeout;
    let desc = describe_selector(&args);

    loop {
        if let Ok(resp) = call_script(s, context, script, args.clone()).await {
            let inner = resp.get("result").and_then(|r| r.get("result"));
            let typ = inner.and_then(|r| r.get("type")).and_then(Value::as_str).unwrap_or("");
            let value = inner.and_then(|r| r.get("value")).and_then(Value::as_str).unwrap_or("");
            if typ == "string" && !value.is_empty() {
                if let Ok(info) = serde_json::from_str::<ElementInfo>(value) {
                    return Ok(info);
                }
            }
        }
        if std::time::Instant::now() > deadline {
            return Err(anyhow!(
                "timeout after {} waiting for '{desc}': element not found",
                crate::errors::format_go_duration(timeout)
            ));
        }
        sleep(Duration::from_millis(100)).await;
    }
}
