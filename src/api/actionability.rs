use std::time::Duration;

use anyhow::anyhow;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::sleep;

use super::handlers_elements::semantic_matches_helper;
use super::helpers::{
    build_el_base_args, build_el_semantic_args, call_script, has_semantic, parse_script_result,
    BoxInfo, ElementInfo, ElementParams,
};
use super::session::Session;

/// A specific actionability check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCheck {
    Visible,
    Stable,
    ReceivesEvents,
    Enabled,
    Editable,
}

// Check sets matching Playwright's actionability matrix.
pub fn click_checks() -> Vec<ActionCheck> {
    vec![
        ActionCheck::Visible,
        ActionCheck::Stable,
        ActionCheck::ReceivesEvents,
        ActionCheck::Enabled,
    ]
}
pub fn hover_checks() -> Vec<ActionCheck> {
    vec![ActionCheck::Visible, ActionCheck::Stable, ActionCheck::ReceivesEvents]
}
pub fn fill_checks() -> Vec<ActionCheck> {
    vec![ActionCheck::Visible, ActionCheck::Enabled, ActionCheck::Editable]
}
pub fn select_checks() -> Vec<ActionCheck> {
    vec![ActionCheck::Visible, ActionCheck::Enabled]
}
pub fn scroll_checks() -> Vec<ActionCheck> {
    vec![ActionCheck::Stable]
}

/// JSON structure returned by the combined actionability script.
#[derive(Debug, Default, Deserialize)]
struct ActionableResult {
    #[serde(default)]
    status: String,
    #[serde(default)]
    check: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    text: String,
    #[serde(default, rename = "box")]
    box_: BoxInfo,
}

fn checks_contain(checks: &[ActionCheck], c: ActionCheck) -> bool {
    checks.contains(&c)
}

/// Builds a synchronous JS function that finds an element, scrolls into view,
/// and runs all applicable actionability checks inline (stability excluded).
fn build_actionable_script(ep: &ElementParams, checks: &[ActionCheck]) -> (String, Vec<Value>) {
    let chk_visible = checks_contain(checks, ActionCheck::Visible);
    let chk_events = checks_contain(checks, ActionCheck::ReceivesEvents);
    let chk_enabled = checks_contain(checks, ActionCheck::Enabled);
    let chk_editable = checks_contain(checks, ActionCheck::Editable);

    if !has_semantic(ep) && !ep.selector.is_empty() {
        return build_css_actionable_script(ep, chk_visible, chk_events, chk_enabled, chk_editable);
    }
    build_semantic_actionable_script(ep, chk_visible, chk_events, chk_enabled, chk_editable)
}

fn build_css_actionable_script(
    ep: &ElementParams,
    chk_visible: bool,
    chk_events: bool,
    chk_enabled: bool,
    chk_editable: bool,
) -> (String, Vec<Value>) {
    let mut args = build_el_base_args(ep);
    args.push(json!({ "type": "boolean", "value": chk_visible }));
    args.push(json!({ "type": "boolean", "value": chk_events }));
    args.push(json!({ "type": "boolean", "value": chk_enabled }));
    args.push(json!({ "type": "boolean", "value": chk_editable }));

    let script = format!(
        r#"
		(scope, selector, index, hasIndex, chkVisible, chkEvents, chkEnabled, chkEditable) => {{
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return JSON.stringify({{status:'not_found'}});
			let el;
			if (hasIndex) {{
				const all = root.querySelectorAll(selector);
				el = all[index];
			}} else {{
				el = root.querySelector(selector);
			}}
			if (!el) return JSON.stringify({{status:'not_found'}});

			if (el.scrollIntoViewIfNeeded) {{
				el.scrollIntoViewIfNeeded(true);
			}} else {{
				el.scrollIntoView({{ block: 'center', inline: 'nearest' }});
			}}
			const rect = el.getBoundingClientRect();
{check_body}
			return JSON.stringify({{
				status:'ok',
				tag: el.tagName.toLowerCase(),
				text: (el.innerText || '').trim(),
				box: {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }}
			}});
		}}
	"#,
        check_body = actionability_check_body()
    );
    (script, args)
}

fn build_semantic_actionable_script(
    ep: &ElementParams,
    chk_visible: bool,
    chk_events: bool,
    chk_enabled: bool,
    chk_editable: bool,
) -> (String, Vec<Value>) {
    let mut args = build_el_semantic_args(ep);
    args.push(json!({ "type": "boolean", "value": chk_visible }));
    args.push(json!({ "type": "boolean", "value": chk_events }));
    args.push(json!({ "type": "boolean", "value": chk_enabled }));
    args.push(json!({ "type": "boolean", "value": chk_editable }));

    let script = format!(
        r#"
		(scope, selector, role, text, label, placeholder, alt, title, testid, xpath, index, hasIndex, chkVisible, chkEvents, chkEnabled, chkEditable) => {{
			const root = scope ? document.querySelector(scope) : document;
			if (!root) return JSON.stringify({{status:'not_found'}});
	{helper}
			const found = collectMatches(root, selector, role, text, label, placeholder, alt, title, testid, xpath);
			let el;
			if (hasIndex) {{
				el = found[index];
			}} else {{
				el = pickBest(found, text);
			}}
			if (!el) return JSON.stringify({{status:'not_found'}});

			if (el.scrollIntoViewIfNeeded) {{
				el.scrollIntoViewIfNeeded(true);
			}} else {{
				el.scrollIntoView({{ block: 'center', inline: 'nearest' }});
			}}
			const rect = el.getBoundingClientRect();
{check_body}
			return JSON.stringify({{
				status:'ok',
				tag: el.tagName.toLowerCase(),
				text: (el.innerText || '').trim(),
				box: {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }}
			}});
		}}
	"#,
        helper = semantic_matches_helper(),
        check_body = actionability_check_body()
    );
    (script, args)
}

/// Shared JS check body appended after element finding + scroll.
fn actionability_check_body() -> &'static str {
    r#"
			if (chkVisible) {
				if (rect.width === 0 || rect.height === 0)
					return JSON.stringify({status:'failed', check:'visible', reason:'zero size'});
				const style = window.getComputedStyle(el);
				if (style.visibility === 'hidden')
					return JSON.stringify({status:'failed', check:'visible', reason:'visibility: hidden'});
				if (style.display === 'none')
					return JSON.stringify({status:'failed', check:'visible', reason:'display: none'});
			}
			if (chkEnabled) {
				if (el.disabled === true)
					return JSON.stringify({status:'failed', check:'enabled', reason:'disabled attribute'});
				if (el.getAttribute('aria-disabled') === 'true')
					return JSON.stringify({status:'failed', check:'enabled', reason:'aria-disabled'});
				const fs = el.closest('fieldset[disabled]');
				if (fs) {
					const legend = fs.querySelector('legend');
					if (!legend || !legend.contains(el))
						return JSON.stringify({status:'failed', check:'enabled', reason:'inside disabled fieldset'});
				}
			}
			if (chkEditable) {
				if (el.readOnly === true)
					return JSON.stringify({status:'failed', check:'editable', reason:'readonly attribute'});
				if (el.getAttribute('aria-readonly') === 'true')
					return JSON.stringify({status:'failed', check:'editable', reason:'aria-readonly'});
				const tag = el.tagName.toLowerCase();
				if (tag === 'input') {
					const t = (el.type || 'text').toLowerCase();
					const textTypes = ['text','password','email','number','search','tel','url'];
					if (!textTypes.includes(t))
						return JSON.stringify({status:'failed', check:'editable', reason:'input type ' + t + ' not editable'});
				} else if (tag !== 'textarea' && !el.isContentEditable) {
					return JSON.stringify({status:'failed', check:'editable', reason:'not a text input element'});
				}
			}
			if (chkEvents) {
				const cx = rect.x + rect.width/2, cy = rect.y + rect.height/2;
				const hit = document.elementFromPoint(cx, cy);
				if (!hit || (el !== hit && !el.contains(hit)))
					return JSON.stringify({status:'failed', check:'receivesEvents', reason:'element is obscured'});
			}
"#
}

/// Runs the combined actionability script and returns the parsed result.
async fn call_actionable_script(
    s: &dyn Session,
    context: &str,
    script: &str,
    args: Vec<Value>,
) -> anyhow::Result<ActionableResult> {
    let resp = call_script(s, context, script, args).await?;
    let val = parse_script_result(&resp)?;
    serde_json::from_str(&val).map_err(|e| anyhow!("failed to parse actionability result: {e}"))
}

/// Polls until the element is found and passes all actionability checks.
pub async fn wait_for_actionable(
    s: &dyn Session,
    context: &str,
    ep: ElementParams,
    checks: &[ActionCheck],
) -> anyhow::Result<ElementInfo> {
    let ep = ep.with_default_timeout();
    let need_stable = checks_contain(checks, ActionCheck::Stable);
    let checks_without_stable: Vec<ActionCheck> =
        checks.iter().copied().filter(|c| *c != ActionCheck::Stable).collect();
    let (script, args) = build_actionable_script(&ep, &checks_without_stable);

    let deadline = std::time::Instant::now() + ep.timeout;
    let mut last_result: Option<ActionableResult> = None;

    loop {
        if let Ok(result) = call_actionable_script(s, context, &script, args.clone()).await {
            if result.status == "ok" {
                if need_stable {
                    sleep(Duration::from_millis(50)).await;
                    if let Ok(result2) = call_actionable_script(s, context, &script, args.clone()).await {
                        if result2.status == "ok" {
                            if result.box_ == result2.box_ {
                                return Ok(ElementInfo {
                                    tag: result2.tag,
                                    text: result2.text,
                                    box_: result2.box_,
                                });
                            }
                            last_result = Some(ActionableResult {
                                status: "failed".to_string(),
                                check: "stable".to_string(),
                                reason: "element is moving or resizing".to_string(),
                                ..Default::default()
                            });
                        }
                    }
                } else {
                    return Ok(ElementInfo {
                        tag: result.tag,
                        text: result.text,
                        box_: result.box_,
                    });
                }
            } else {
                last_result = Some(result);
            }
        }

        if std::time::Instant::now() > deadline {
            let timeout = crate::errors::format_go_duration(ep.timeout);
            if let Some(lr) = &last_result {
                if lr.status == "not_found" {
                    return Err(anyhow!("timeout after {timeout}: element not found"));
                }
                return Err(anyhow!(
                    "timeout after {timeout}: {} check failed — {}",
                    lr.check,
                    lr.reason
                ));
            }
            return Err(anyhow!("timeout after {timeout} waiting for element"));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

/// Resolves an element with actionability checks. Falls back to plain
/// resolve_element when forced or no checks are needed.
pub async fn resolve_with_actionability(
    s: &dyn Session,
    context: &str,
    ep: ElementParams,
    checks: &[ActionCheck],
) -> anyhow::Result<ElementInfo> {
    if ep.force || checks.is_empty() {
        return super::helpers::resolve_element(s, context, ep).await;
    }
    let info = wait_for_actionable(s, context, ep, checks).await?;
    s.set_last_element_box(info.box_);
    Ok(info)
}
