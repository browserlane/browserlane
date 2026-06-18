//! Phase 3 foundation: the shared find machinery (semantic match helper, find
//! script builders, find/findAll routes). The element state/interaction handlers
//! (text/html/click/...) are ported in their respective clusters.

use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use serde_json::{json, Map, Value};
use tokio::time::sleep;

use super::helpers::wait_for_element_with_script;
use super::router::{BidiCommand, BrowserSession, Router, DEFAULT_TIMEOUT};
use super::session::new_api_session;

impl Router {
    /// Handles `browserlane:element.find` / `browserlane:page.find` with wait-for-selector.
    pub(crate) async fn handle_browserlane_find(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let timeout = param_timeout(&cmd.params);
        let (script, args) = build_find_script(&cmd.params, false);

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match wait_for_element_with_script(&s, &context, &script, args, timeout).await {
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

    /// Handles `browserlane:element.findAll` / `browserlane:page.findAll` — returns all matches.
    pub(crate) async fn handle_browserlane_find_all(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let timeout = param_timeout(&cmd.params);
        let has_text = cmd.params.get("hasText").and_then(Value::as_str).unwrap_or("");
        let has = cmd.params.get("has").and_then(Value::as_str).unwrap_or("");

        let (script, mut args) = build_find_script(&cmd.params, true);
        args.push(json!({ "type": "string", "value": has_text }));
        args.push(json!({ "type": "string", "value": has }));

        match self.wait_for_elements(session, &context, &script, args, timeout).await {
            Ok(elements) => {
                let count = elements.len();
                self.send_success(session, cmd.id, json!({ "elements": elements, "count": count }));
            }
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// Polls until at least one matching element is found, then returns all.
    async fn wait_for_elements(
        &self,
        session: &Arc<BrowserSession>,
        context: &str,
        script: &str,
        args: Vec<Value>,
        timeout: Duration,
    ) -> anyhow::Result<Vec<Value>> {
        let deadline = std::time::Instant::now() + timeout;
        let desc = describe_selector(&args);

        loop {
            let params = json!({
                "functionDeclaration": script,
                "target": { "context": context },
                "arguments": args,
                "awaitPromise": false,
                "resultOwnership": "root",
            });

            if let Ok(resp) = self.send_internal_command(session, "script.callFunction", params).await {
                let inner = resp.get("result").and_then(|r| r.get("result"));
                let typ = inner.and_then(|r| r.get("type")).and_then(Value::as_str).unwrap_or("");
                let value = inner.and_then(|r| r.get("value")).and_then(Value::as_str).unwrap_or("");
                if typ == "string" && !value.is_empty() {
                    if let Ok(elements) = serde_json::from_str::<Vec<Value>>(value) {
                        if !elements.is_empty() {
                            return Ok(elements);
                        }
                    }
                }
            }

            if std::time::Instant::now() > deadline {
                return Err(anyhow!(
                    "timeout after {} waiting for '{desc}': no elements found",
                    crate::errors::format_go_duration(timeout)
                ));
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Extracts a millisecond "timeout" param, defaulting to DEFAULT_TIMEOUT.
fn param_timeout(params: &Map<String, Value>) -> Duration {
    match params.get("timeout").and_then(Value::as_f64) {
        Some(ms) if ms > 0.0 => Duration::from_millis(ms as u64),
        _ => DEFAULT_TIMEOUT,
    }
}

/// Builds the JS function and arguments for element finding.
pub fn build_find_script(params: &Map<String, Value>, find_all: bool) -> (String, Vec<Value>) {
    let s = |k: &str| params.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let selector = s("selector");
    let scope = s("scope");
    let role = s("role");
    let text = s("text");
    let label = s("label");
    let placeholder = s("placeholder");
    let alt = s("alt");
    let title = s("title");
    let testid = s("testid");
    let xpath = s("xpath");

    let mut args = vec![json!({ "type": "string", "value": scope })];

    let has_semantic = !role.is_empty()
        || !text.is_empty()
        || !label.is_empty()
        || !placeholder.is_empty()
        || !alt.is_empty()
        || !title.is_empty()
        || !testid.is_empty()
        || !xpath.is_empty();

    if !has_semantic && !selector.is_empty() {
        args.push(json!({ "type": "string", "value": selector }));
        let script = if find_all {
            build_css_find_all_script()
        } else {
            build_css_find_script()
        };
        return (script, args);
    }

    args.push(json!({ "type": "string", "value": selector }));
    args.push(json!({ "type": "string", "value": role }));
    args.push(json!({ "type": "string", "value": text }));
    args.push(json!({ "type": "string", "value": label }));
    args.push(json!({ "type": "string", "value": placeholder }));
    args.push(json!({ "type": "string", "value": alt }));
    args.push(json!({ "type": "string", "value": title }));
    args.push(json!({ "type": "string", "value": testid }));
    args.push(json!({ "type": "string", "value": xpath }));

    let script = if find_all {
        build_semantic_find_all_script()
    } else {
        build_semantic_find_script()
    };
    (script, args)
}

fn build_css_find_script() -> String {
    r#"
		(scope, selector) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return null;
			const el = root.querySelector(selector);
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
    .to_string()
}

fn build_css_find_all_script() -> String {
    r#"
		(scope, selector, hasText, has) => {
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return '[]';
			let els = Array.from(root.querySelectorAll(selector));
			if (hasText) {
				els = els.filter(el => (el.textContent || '').includes(hasText));
			}
			if (has) {
				els = els.filter(el => el.querySelector(has) !== null);
			}
			return JSON.stringify(els.map((el, i) => {
				const rect = el.getBoundingClientRect();
				return {
					tag: el.tagName.toLowerCase(),
					text: (el.innerText || '').trim(),
					box: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
					index: i
				};
			}));
		}
	"#
    .to_string()
}

/// The shared semantic-match JS helper (collectMatches, pickBest, toInfo, etc.).
pub fn semantic_matches_helper() -> &'static str {
    r#"
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

			function matches(el, selector, role, text, label, placeholder, alt, title, testid) {
				if (selector && !el.matches(selector)) return false;
				if (role) {
					if (getImplicitRole(el) !== role.toLowerCase()) return false;
				}
				if (text) {
					const elText = (el.textContent || '').trim();
					if (!elText.includes(text)) return false;
				}
				if (label) {
					const ariaLabel = el.getAttribute('aria-label') || '';
					const labelledBy = el.getAttribute('aria-labelledby');
					let labelText = ariaLabel;
					if (labelledBy) {
						const labelEl = document.getElementById(labelledBy);
						if (labelEl) labelText = labelText || (labelEl.textContent || '').trim();
					}
					if (el.id) {
						const assocLabel = document.querySelector('label[for="' + el.id + '"]');
						if (assocLabel) labelText = labelText || (assocLabel.textContent || '').trim();
					}
					if (!labelText.includes(label)) return false;
				}
				if (placeholder && el.getAttribute('placeholder') !== placeholder) return false;
				if (alt && el.getAttribute('alt') !== alt) return false;
				if (title && el.getAttribute('title') !== title) return false;
				if (testid && el.getAttribute('data-testid') !== testid) return false;
				return true;
			}

			function toInfo(el) {
				const rect = el.getBoundingClientRect();
				return {
					tag: el.tagName.toLowerCase(),
					text: (el.innerText || '').trim(),
					box: { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
				};
			}

			function collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath) {
				const found = [];
				if (xpath) {
					const xr = document.evaluate(xpath, root, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
					for (let i = 0; i < xr.snapshotLength; i++) {
						const el = xr.snapshotItem(i);
						if (el && el.nodeType === 1 && matches(el, selector, role, text, label, placeholder, alt, title, testid)) {
							found.push(el);
						}
					}
				} else {
					const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT);
					let node;
					while (node = walker.nextNode()) {
						if (matches(node, selector, role, text, label, placeholder, alt, title, testid)) {
							found.push(node);
						}
					}
				}
				return found;
			}

			function pickBest(found, text) {
				if (found.length === 0) return null;
				if (!text || found.length === 1) return found[0];
				let best = found[0];
				let bestLen = (best.textContent || '').length;
				for (let i = 1; i < found.length; i++) {
					const len = (found[i].textContent || '').length;
					if (len < bestLen) {
						best = found[i];
						bestLen = len;
					}
				}
				return best;
			}
	"#
}

fn build_semantic_find_script() -> String {
    format!(
        r#"
		(scope, selector, role, text, label, placeholder, alt, title, testid, xpath) => {{
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return null;
{helper}
			const found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
			const best = pickBest(found, text);
			if (!best) return null;
			if (best.scrollIntoViewIfNeeded) {{
				best.scrollIntoViewIfNeeded(true);
			}} else {{
				best.scrollIntoView({{ block: 'center', inline: 'nearest' }});
			}}
			return JSON.stringify(toInfo(best));
		}}
	"#,
        helper = semantic_matches_helper()
    )
}

fn build_semantic_find_all_script() -> String {
    format!(
        r#"
		(scope, selector, role, text, label, placeholder, alt, title, testid, xpath, hasText, has) => {{
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return '[]';
{helper}
			let found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
			if (hasText) {{
				found = found.filter(el => (el.textContent || '').includes(hasText));
			}}
			if (has) {{
				found = found.filter(el => el.querySelector(has) !== null);
			}}
			return JSON.stringify(found.map((el, i) => {{
				const info = toInfo(el);
				info.index = i;
				return info;
			}}));
		}}
	"#,
        helper = semantic_matches_helper()
    )
}

/// Builds a human-readable description of the selector for error messages.
pub fn describe_selector(args: &[Value]) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = args.get(1).and_then(|a| a.get("value")).and_then(Value::as_str) {
        if !v.is_empty() {
            parts.push(v.to_string());
        }
    }
    let labels = ["role", "text", "label", "placeholder", "alt", "title", "testid", "xpath"];
    for (i, lbl) in labels.iter().enumerate() {
        let idx = i + 2;
        if let Some(v) = args.get(idx).and_then(|a| a.get("value")).and_then(Value::as_str) {
            if !v.is_empty() {
                parts.push(format!("{lbl}={v}"));
            }
        }
    }
    if parts.is_empty() {
        return "element".to_string();
    }
    parts.join(", ")
}
