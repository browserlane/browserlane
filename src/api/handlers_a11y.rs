//! Phase 3 a11y cluster: browserlane:element.role / element.label (computed ARIA role
//! + accessible name) and browserlane:page.a11yTree (the full accessibility tree).

use std::sync::Arc;

use anyhow::anyhow;
use serde_json::{json, Value};

use super::handlers_state::{build_el_state_script, eval_element_script};
use super::helpers::{extract_element_params, parse_script_result};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::{new_api_session, Session};

impl Router {
    /// `browserlane:element.role` — returns the element's computed ARIA role.
    pub(crate) async fn handle_browserlane_el_role(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_state_script(&ep, ROLE_EXPR);
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_element_script(&s, &context, &script, args).await {
            Ok(val) => self.send_success(session, cmd.id, json!({ "role": val })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// `browserlane:element.label` — returns the element's accessible name.
    pub(crate) async fn handle_browserlane_el_label(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let ep = extract_element_params(&cmd.params);
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let (script, args) = build_el_state_script(&ep, LABEL_EXPR);
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        match eval_element_script(&s, &context, &script, args).await {
            Ok(val) => self.send_success(session, cmd.id, json!({ "label": val })),
            Err(e) => self.send_error(session, cmd.id, &e),
        }
    }

    /// `browserlane:page.a11yTree` — returns the accessibility tree.
    pub(crate) async fn handle_browserlane_page_a11y_tree(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let interesting_only = match cmd.params.get("everything").and_then(Value::as_bool) {
            Some(v) => !v,
            None => true,
        };
        let root_selector = cmd.params.get("root").and_then(Value::as_str).unwrap_or("");

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let tree = match a11y_tree(&s, &context, interesting_only, root_selector).await {
            Ok(t) => t,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let parsed: Value = match serde_json::from_str(&tree) {
            Ok(p) => p,
            Err(e) => return self.send_error(session, cmd.id, &anyhow!("a11yTree parse failed: {e}")),
        };

        self.send_success(session, cmd.id, json!({ "tree": parsed }));
    }
}

/// Calls the a11y tree script in the browser and returns the JSON string result.
pub async fn a11y_tree(
    s: &dyn Session,
    context: &str,
    interesting_only: bool,
    root_selector: &str,
) -> anyhow::Result<String> {
    let args = json!([
        { "type": "boolean", "value": interesting_only },
        { "type": "string", "value": root_selector },
    ]);

    let resp = s
        .send_bidi_command(
            "script.callFunction",
            json!({
                "functionDeclaration": A11Y_TREE_SCRIPT,
                "target": { "context": context },
                "arguments": args,
                "awaitPromise": false,
                "resultOwnership": "root",
            }),
        )
        .await
        .map_err(|e| anyhow!("a11yTree failed: {e}"))?;

    parse_script_result(&resp).map_err(|e| anyhow!("a11yTree failed: {e}"))
}

/// JS expression for `element.role` — computed ARIA role.
const ROLE_EXPR: &str = r#"(() => {
	if (typeof el.computedRole === 'string' && el.computedRole !== '') return el.computedRole;
	const explicit = el.getAttribute('role');
	if (explicit) return explicit.toLowerCase();
	const IMPLICIT_ROLES = {
		A: (e) => e.hasAttribute('href') ? 'link' : '',
		AREA: (e) => e.hasAttribute('href') ? 'link' : '',
		ARTICLE: () => 'article', ASIDE: () => 'complementary',
		BUTTON: () => 'button', DETAILS: () => 'group', DIALOG: () => 'dialog',
		FOOTER: () => 'contentinfo', FORM: () => 'form',
		H1: () => 'heading', H2: () => 'heading', H3: () => 'heading',
		H4: () => 'heading', H5: () => 'heading', H6: () => 'heading',
		HEADER: () => 'banner', HR: () => 'separator',
		IMG: (e) => e.getAttribute('alt') ? 'img' : 'presentation',
		INPUT: (e) => {
			const t = (e.getAttribute('type') || 'text').toLowerCase();
			const m = {button:'button',checkbox:'checkbox',image:'button',
				number:'spinbutton',radio:'radio',range:'slider',
				reset:'button',search:'searchbox',submit:'button',text:'textbox',
				email:'textbox',tel:'textbox',url:'textbox',password:'textbox'};
			return m[t] || 'textbox';
		},
		LI: () => 'listitem', MAIN: () => 'main', MENU: () => 'list',
		NAV: () => 'navigation', OL: () => 'list', OPTION: () => 'option',
		OUTPUT: () => 'status', PROGRESS: () => 'progressbar',
		SECTION: () => 'region',
		SELECT: (e) => e.hasAttribute('multiple') ? 'listbox' : 'combobox',
		SUMMARY: () => 'button', TABLE: () => 'table',
		TBODY: () => 'rowgroup', THEAD: () => 'rowgroup', TFOOT: () => 'rowgroup',
		TD: () => 'cell', TEXTAREA: () => 'textbox', TH: () => 'columnheader',
		TR: () => 'row', UL: () => 'list',
	};
	const fn = IMPLICIT_ROLES[el.tagName];
	return fn ? fn(el) : '';
})()"#;

/// JS expression for `element.label` — accessible name.
const LABEL_EXPR: &str = r#"(() => {
	if (typeof el.computedName === 'string' && el.computedName !== '') return el.computedName;
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
	const parentLabel = el.closest('label');
	if (parentLabel) return (parentLabel.textContent || '').trim();
	const placeholder = el.getAttribute('placeholder');
	if (placeholder) return placeholder;
	const alt = el.getAttribute('alt');
	if (alt) return alt;
	const title = el.getAttribute('title');
	if (title) return title;
	const text = (el.textContent || '').trim();
	if (text) return text;
	return '';
})()"#;

/// The JS function that builds the accessibility tree.
const A11Y_TREE_SCRIPT: &str = r#"(interestingOnly, rootSelector) => {
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

	function getRole(el) {
		if (typeof el.computedRole === 'string' && el.computedRole !== '') return el.computedRole;
		const explicit = el.getAttribute('role');
		if (explicit) return explicit.toLowerCase();
		const fn = IMPLICIT_ROLES[el.tagName];
		return fn ? fn(el) : 'generic';
	}

	function getName(el) {
		if (typeof el.computedName === 'string') return el.computedName;
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
		const placeholder = el.getAttribute('placeholder');
		if (placeholder) return placeholder;
		const alt = el.getAttribute('alt');
		if (alt) return alt;
		const title = el.getAttribute('title');
		if (title) return title;
		return '';
	}

	function getChildren(el) {
		if (el.shadowRoot) return Array.from(el.shadowRoot.children);
		return Array.from(el.children);
	}

	function getHeadingLevel(el) {
		const tag = el.tagName;
		if (tag === 'H1') return 1;
		if (tag === 'H2') return 2;
		if (tag === 'H3') return 3;
		if (tag === 'H4') return 4;
		if (tag === 'H5') return 5;
		if (tag === 'H6') return 6;
		const level = el.getAttribute('aria-level');
		if (level) return parseInt(level, 10);
		return undefined;
	}

	function buildNode(el) {
		const role = getRole(el);
		const name = getName(el);

		// Collect children first
		const childNodes = [];
		for (const child of getChildren(el)) {
			if (child.nodeType !== 1) continue;
			const nodes = buildNode(child);
			if (nodes) {
				if (Array.isArray(nodes)) {
					childNodes.push(...nodes);
				} else {
					childNodes.push(nodes);
				}
			}
		}

		// If interestingOnly, skip uninteresting nodes (promote their children)
		if (interestingOnly) {
			if (role === 'none' || role === 'presentation') {
				return childNodes.length ? childNodes : null;
			}
			if (role === 'generic' && !name) {
				return childNodes.length ? childNodes : null;
			}
		}

		const node = { role: role };
		if (name) node.name = name;

		// Collect states
		if (el.hasAttribute('disabled') || el.disabled) node.disabled = true;
		if (el.hasAttribute('aria-expanded')) node.expanded = el.getAttribute('aria-expanded') === 'true';
		if (document.activeElement === el) node.focused = true;

		// checked (checkbox, radio, aria-checked)
		if (typeof el.checked === 'boolean' && (el.type === 'checkbox' || el.type === 'radio')) {
			node.checked = el.checked;
		} else if (el.hasAttribute('aria-checked')) {
			const v = el.getAttribute('aria-checked');
			node.checked = v === 'true' ? true : v === 'mixed' ? 'mixed' : false;
		}

		// pressed (toggle buttons)
		if (el.hasAttribute('aria-pressed')) {
			const v = el.getAttribute('aria-pressed');
			node.pressed = v === 'true' ? true : v === 'mixed' ? 'mixed' : false;
		}

		if (el.hasAttribute('aria-selected') && el.getAttribute('aria-selected') === 'true') node.selected = true;
		if (el.hasAttribute('required') || el.required) node.required = true;
		if (el.hasAttribute('readonly') || el.readOnly) node.readonly = true;

		// heading level
		const level = getHeadingLevel(el);
		if (level !== undefined) node.level = level;

		// value for range controls
		if (el.hasAttribute('aria-valuetext')) {
			node.value = el.getAttribute('aria-valuetext');
		} else if (el.hasAttribute('aria-valuenow')) {
			node.value = parseFloat(el.getAttribute('aria-valuenow'));
		} else if ((el.tagName === 'INPUT' && (el.type === 'range' || el.type === 'number')) || el.tagName === 'PROGRESS') {
			node.value = el.value !== undefined && el.value !== '' ? parseFloat(el.value) : undefined;
		}

		if (el.hasAttribute('aria-valuemin')) node.valuemin = parseFloat(el.getAttribute('aria-valuemin'));
		if (el.hasAttribute('aria-valuemax')) node.valuemax = parseFloat(el.getAttribute('aria-valuemax'));

		// description
		const describedBy = el.getAttribute('aria-describedby');
		if (describedBy) {
			const parts = describedBy.split(/\s+/).map(id => {
				const ref = document.getElementById(id);
				return ref ? (ref.textContent || '').trim() : '';
			}).filter(Boolean);
			if (parts.length) node.description = parts.join(' ');
		}

		if (childNodes.length) node.children = childNodes;

		return node;
	}

	const rootEl = rootSelector ? document.querySelector(rootSelector) : document.body;
	if (!rootEl) return JSON.stringify({role: 'WebArea', name: document.title, children: []});

	const children = [];
	for (const child of getChildren(rootEl)) {
		if (child.nodeType !== 1) continue;
		const nodes = buildNode(child);
		if (nodes) {
			if (Array.isArray(nodes)) {
				children.push(...nodes);
			} else {
				children.push(nodes);
			}
		}
	}

	return JSON.stringify({
		role: 'WebArea',
		name: document.title,
		children: children
	});
}"#;
