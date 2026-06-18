//! Phase 3 storage cluster: cookie + localStorage/sessionStorage + init-script
//! routes plus the exported standalone cookie functions used by the MCP agent.

use std::sync::Arc;

use anyhow::anyhow;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::helpers::{check_bidi_error, eval_simple_script};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::{new_api_session, Session};

// --- BiDi cookie types ---

/// A BiDi BytesValue: `{type: "string", value: "..."}`.
#[derive(Debug, Default, Clone, Deserialize)]
struct BidiCookieValue {
    #[serde(default)]
    value: String,
}

/// A cookie from a storage.getCookies response.
#[derive(Debug, Default, Clone, Deserialize)]
struct BidiCookie {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: BidiCookieValue,
    #[serde(default)]
    domain: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    size: i64,
    #[serde(rename = "httpOnly", default)]
    http_only: bool,
    #[serde(default)]
    secure: bool,
    #[serde(rename = "sameSite", default)]
    same_site: String,
    #[serde(default)]
    expiry: Option<Value>,
}

// --- Handlers ---

impl Router {
    /// Handles `browserlane:context.cookies` — returns cookies for the user context.
    pub(crate) async fn handle_context_cookies(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let mut cookies = match self.get_cookies_for_context(session, user_context).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        if let Some(urls_raw) = cmd.params.get("urls") {
            if let Some(url_slice) = urls_raw.as_array() {
                if !url_slice.is_empty() {
                    let urls: Vec<String> = url_slice
                        .iter()
                        .filter_map(|u| u.as_str().map(String::from))
                        .collect();
                    cookies = filter_cookies_by_urls(cookies, &urls);
                }
            }
        }

        self.send_success(session, cmd.id, json!({ "cookies": cookies }));
    }

    /// Handles `browserlane:context.setCookies` — sets cookies via storage.setCookie (one at a time).
    pub(crate) async fn handle_context_set_cookies(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let cookies_raw = match cmd.params.get("cookies").and_then(Value::as_array) {
            Some(c) if !c.is_empty() => c.clone(),
            _ => return self.send_error(session, cmd.id, &anyhow!("cookies array is required")),
        };

        for c_raw in &cookies_raw {
            let c = match c_raw.as_object() {
                Some(c) => c,
                None => continue,
            };

            let bidi_cookie = match build_set_cookie(c) {
                Ok(bc) => bc,
                Err(e) => return self.send_error(session, cmd.id, &e),
            };

            let params = json!({
                "cookie": bidi_cookie,
                "partition": { "type": "storageKey", "userContext": user_context },
            });

            let resp = match self.send_internal_command(session, "storage.setCookie", params).await {
                Ok(r) => r,
                Err(e) => return self.send_error(session, cmd.id, &e),
            };
            if let Err(e) = check_bidi_error(&resp) {
                return self.send_error(session, cmd.id, &e);
            }
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:context.clearCookies` — deletes all cookies for the user context.
    pub(crate) async fn handle_context_clear_cookies(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let mut params = json!({
            "partition": { "type": "storageKey", "userContext": user_context },
        });
        if let Some(filter) = cmd.params.get("filter").filter(|v| v.is_object()) {
            params["filter"] = filter.clone();
        }

        let resp = match self.send_internal_command(session, "storage.deleteCookies", params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:context.storage` — returns cookies + localStorage + sessionStorage.
    pub(crate) async fn handle_context_storage(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let cookies = match self.get_cookies_for_context(session, user_context).await {
            Ok(c) => c,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };

        let context = match self.find_context_for_user_context(session, user_context).await {
            Ok(c) => c,
            Err(_) => {
                return self.send_success(
                    session,
                    cmd.id,
                    json!({ "cookies": cookies, "origins": [] }),
                )
            }
        };

        let storage_script = r#"() => {
		const ls = {};
		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i);
			ls[key] = localStorage.getItem(key);
		}
		const ss = {};
		for (let i = 0; i < sessionStorage.length; i++) {
			const key = sessionStorage.key(i);
			ss[key] = sessionStorage.getItem(key);
		}
		return JSON.stringify({ origin: location.origin, localStorage: ls, sessionStorage: ss });
	}"#;

        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        let storage_json = match eval_simple_script(&s, &context, storage_script).await {
            Ok(j) => j,
            Err(_) => {
                return self.send_success(
                    session,
                    cmd.id,
                    json!({ "cookies": cookies, "origins": [] }),
                )
            }
        };

        #[derive(Deserialize)]
        struct StorageData {
            #[serde(default)]
            origin: String,
            #[serde(rename = "localStorage", default)]
            local_storage: Map<String, Value>,
            #[serde(rename = "sessionStorage", default)]
            session_storage: Map<String, Value>,
        }
        let data: StorageData = match serde_json::from_str(&storage_json) {
            Ok(d) => d,
            Err(e) => {
                return self.send_error(session, cmd.id, &anyhow!("failed to parse storage data: {e}"))
            }
        };

        let ls_items: Vec<Value> = data
            .local_storage
            .iter()
            .map(|(k, v)| json!({ "name": k, "value": v.as_str().unwrap_or("") }))
            .collect();
        let ss_items: Vec<Value> = data
            .session_storage
            .iter()
            .map(|(k, v)| json!({ "name": k, "value": v.as_str().unwrap_or("") }))
            .collect();

        let origins = json!([{
            "origin": data.origin,
            "localStorage": ls_items,
            "sessionStorage": ss_items,
        }]);

        self.send_success(session, cmd.id, json!({ "cookies": cookies, "origins": origins }));
    }

    /// Handles `browserlane:context.setStorage` — restores cookies + localStorage + sessionStorage.
    pub(crate) async fn handle_context_set_storage(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let state = match cmd.params.get("state").and_then(Value::as_object) {
            Some(s) => s.clone(),
            None => return self.send_error(session, cmd.id, &anyhow!("state is required")),
        };

        // 1. Set cookies.
        if let Some(cookies_raw) = state.get("cookies").and_then(Value::as_array) {
            if !cookies_raw.is_empty() {
                for c_raw in cookies_raw {
                    let c = match c_raw.as_object() {
                        Some(c) => c,
                        None => continue,
                    };
                    let bidi_cookie = match build_set_cookie(c) {
                        Ok(bc) => bc,
                        Err(_) => continue,
                    };
                    let params = json!({
                        "cookie": bidi_cookie,
                        "partition": { "type": "storageKey", "userContext": user_context },
                    });
                    let resp = match self.send_internal_command(session, "storage.setCookie", params).await {
                        Ok(r) => r,
                        Err(e) => return self.send_error(session, cmd.id, &e),
                    };
                    if let Err(e) = check_bidi_error(&resp) {
                        return self.send_error(session, cmd.id, &e);
                    }
                }
            }
        }

        // 2. Set localStorage/sessionStorage from origins.
        if let Some(origins_raw) = state.get("origins").and_then(Value::as_array) {
            if !origins_raw.is_empty() {
                if let Ok(context) = self.find_context_for_user_context(session, user_context).await {
                    for o_raw in origins_raw {
                        let o = match o_raw.as_object() {
                            Some(o) => o,
                            None => continue,
                        };
                        let ls_items = o.get("localStorage").and_then(Value::as_array);
                        let ss_items = o.get("sessionStorage").and_then(Value::as_array);

                        let ls_empty = ls_items.map(|v| v.is_empty()).unwrap_or(true);
                        let ss_empty = ss_items.map(|v| v.is_empty()).unwrap_or(true);
                        if ls_empty && ss_empty {
                            continue;
                        }

                        let ls_json = serde_json::to_string(&ls_items.cloned().unwrap_or_default())
                            .unwrap_or_else(|_| "[]".to_string());
                        let ss_json = serde_json::to_string(&ss_items.cloned().unwrap_or_default())
                            .unwrap_or_else(|_| "[]".to_string());

                        let script = format!(
                            "() => {{\n\t\t\t\t\tvar ls = {ls_json};\n\t\t\t\t\tfor (var i = 0; i < ls.length; i++) {{\n\t\t\t\t\t\tlocalStorage.setItem(ls[i].name, ls[i].value);\n\t\t\t\t\t}}\n\t\t\t\t\tvar ss = {ss_json};\n\t\t\t\t\tfor (var i = 0; i < ss.length; i++) {{\n\t\t\t\t\t\tsessionStorage.setItem(ss[i].name, ss[i].value);\n\t\t\t\t\t}}\n\t\t\t\t\treturn 'ok';\n\t\t\t\t}}"
                        );

                        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
                        let _ = eval_simple_script(&s, &context, &script).await;
                    }
                }
            }
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:context.clearStorage` — clears cookies + localStorage + sessionStorage.
    pub(crate) async fn handle_context_clear_storage(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let params = json!({
            "partition": { "type": "storageKey", "userContext": user_context },
        });
        let resp = match self.send_internal_command(session, "storage.deleteCookies", params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }

        if let Ok(context) = self.find_context_for_user_context(session, user_context).await {
            let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
            let _ = eval_simple_script(
                &s,
                &context,
                "() => {\n\t\t\tlocalStorage.clear();\n\t\t\tsessionStorage.clear();\n\t\t\treturn 'ok';\n\t\t}",
            )
            .await;
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Handles `browserlane:context.addInitScript` — adds a preload script scoped to the user context.
    pub(crate) async fn handle_context_add_init_script(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let user_context = cmd.params.get("userContext").and_then(Value::as_str).unwrap_or("");
        if user_context.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("userContext is required"));
        }

        let script = cmd.params.get("script").and_then(Value::as_str).unwrap_or("");
        if script.is_empty() {
            return self.send_error(session, cmd.id, &anyhow!("script is required"));
        }

        let wrapped_script = format!("() => {{ {script} }}");

        let params = json!({
            "functionDeclaration": wrapped_script,
            "userContexts": [user_context],
        });

        let resp = match self.send_internal_command(session, "script.addPreloadScript", params).await {
            Ok(r) => r,
            Err(e) => return self.send_error(session, cmd.id, &e),
        };
        if let Err(e) = check_bidi_error(&resp) {
            return self.send_error(session, cmd.id, &e);
        }

        let script_id = resp
            .get("result")
            .and_then(|r| r.get("script"))
            .and_then(Value::as_str)
            .unwrap_or("");
        self.send_success(session, cmd.id, json!({ "script": script_id }));
    }

    // --- Helper methods ---

    /// Fetches and normalizes cookies for a user context.
    async fn get_cookies_for_context(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        user_context: &str,
    ) -> anyhow::Result<Vec<Value>> {
        let params = json!({
            "partition": { "type": "storageKey", "userContext": user_context },
        });

        let resp = self.send_internal_command(session, "storage.getCookies", params).await?;
        check_bidi_error(&resp)?;

        #[derive(Deserialize)]
        struct Inner {
            #[serde(default)]
            cookies: Vec<BidiCookie>,
        }
        #[derive(Deserialize)]
        struct Outer {
            result: Inner,
        }
        let parsed: Outer = serde_json::from_value(resp)
            .map_err(|e| anyhow!("failed to parse getCookies response: {e}"))?;

        Ok(normalize_cookies(&parsed.result.cookies))
    }

    /// Finds a browsing context (page) in the given user context.
    async fn find_context_for_user_context(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        user_context: &str,
    ) -> anyhow::Result<String> {
        let resp = self
            .send_internal_command(session, "browsingContext.getTree", json!({}))
            .await?;

        #[derive(Deserialize)]
        struct Ctx {
            #[serde(default)]
            context: String,
            #[serde(rename = "userContext", default)]
            user_context: String,
        }
        #[derive(Deserialize)]
        struct Inner {
            #[serde(default)]
            contexts: Vec<Ctx>,
        }
        #[derive(Deserialize)]
        struct Outer {
            result: Inner,
        }
        let parsed: Outer = serde_json::from_value(resp)
            .map_err(|e| anyhow!("failed to parse getTree response: {e}"))?;

        for ctx in parsed.result.contexts {
            if ctx.user_context == user_context {
                return Ok(ctx.context);
            }
        }
        Err(anyhow!("no browsing context found for user context {user_context}"))
    }
}

/// Builds a BiDi cookie object for storage.setCookie from a raw input map.
/// Returns an error when name/domain (or url) is missing.
fn build_set_cookie(c: &Map<String, Value>) -> anyhow::Result<Value> {
    let name = c.get("name").and_then(Value::as_str).unwrap_or("");
    let value = c.get("value").and_then(Value::as_str).unwrap_or("");
    let mut domain = c.get("domain").and_then(Value::as_str).unwrap_or("").to_string();

    if domain.is_empty() {
        if let Some(url_str) = c.get("url").and_then(Value::as_str) {
            if !url_str.is_empty() {
                if let Some(host) = hostname_from_url(url_str) {
                    domain = host;
                }
            }
        }
    }

    if name.is_empty() || domain.is_empty() {
        return Err(anyhow!("cookie name and domain (or url) are required"));
    }

    let mut cookie = Map::new();
    cookie.insert("name".to_string(), json!(name));
    cookie.insert("value".to_string(), json!({ "type": "string", "value": value }));
    cookie.insert("domain".to_string(), json!(domain));
    cookie.insert("path".to_string(), json!("/"));

    if let Some(path) = c.get("path").and_then(Value::as_str) {
        if !path.is_empty() {
            cookie.insert("path".to_string(), json!(path));
        }
    }
    if let Some(http_only) = c.get("httpOnly").and_then(Value::as_bool) {
        cookie.insert("httpOnly".to_string(), json!(http_only));
    }
    if let Some(secure) = c.get("secure").and_then(Value::as_bool) {
        cookie.insert("secure".to_string(), json!(secure));
    }
    if let Some(same_site) = c.get("sameSite").and_then(Value::as_str) {
        if !same_site.is_empty() {
            cookie.insert("sameSite".to_string(), json!(same_site));
        }
    }
    if let Some(expiry) = c.get("expiry").and_then(Value::as_f64) {
        cookie.insert("expiry".to_string(), json!(expiry as i64));
    }

    Ok(Value::Object(cookie))
}

/// Extracts the hostname from a URL string (mirrors Go's url.Parse().Hostname()).
fn hostname_from_url(url_str: &str) -> Option<String> {
    let rest = match url_str.split_once("://") {
        Some((_, r)) => r,
        None => url_str,
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Strip userinfo.
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    // Strip port (handle IPv6 brackets).
    let host = if let Some(stripped) = host_port.strip_prefix('[') {
        stripped.split(']').next().unwrap_or("").to_string()
    } else {
        host_port.split(':').next().unwrap_or("").to_string()
    };
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// Converts BiDi cookie objects to plain maps with string values.
fn normalize_cookies(bidi_cookies: &[BidiCookie]) -> Vec<Value> {
    bidi_cookies
        .iter()
        .map(|c| {
            // Insert keys in Go's sorted-map order for byte-stable JSON.
            let mut m = Map::new();
            m.insert("domain".to_string(), json!(c.domain));
            if let Some(expiry) = &c.expiry {
                m.insert("expiry".to_string(), expiry.clone());
            }
            m.insert("httpOnly".to_string(), json!(c.http_only));
            m.insert("name".to_string(), json!(c.name));
            m.insert("path".to_string(), json!(c.path));
            m.insert("sameSite".to_string(), json!(c.same_site));
            m.insert("secure".to_string(), json!(c.secure));
            m.insert("size".to_string(), json!(c.size));
            m.insert("value".to_string(), json!(c.value.value));
            Value::Object(m)
        })
        .collect()
}

/// Filters cookies to only those matching the given URLs.
fn filter_cookies_by_urls(cookies: Vec<Value>, urls: &[String]) -> Vec<Value> {
    let parsed: Vec<(String, String)> = urls
        .iter()
        .filter_map(|u| {
            let host = hostname_from_url(u)?;
            let path = path_from_url(u);
            Some((host, path))
        })
        .collect();

    cookies
        .into_iter()
        .filter(|cookie| {
            let domain = cookie.get("domain").and_then(Value::as_str).unwrap_or("");
            let path = cookie.get("path").and_then(Value::as_str).unwrap_or("");
            parsed
                .iter()
                .any(|(host, url_path)| domain_matches(domain, host) && path_matches(path, url_path))
        })
        .collect()
}

/// Extracts the path component from a URL string.
fn path_from_url(url_str: &str) -> String {
    let rest = match url_str.split_once("://") {
        Some((_, r)) => r,
        None => url_str,
    };
    match rest.find('/') {
        Some(idx) => {
            let after = &rest[idx..];
            after.split(['?', '#']).next().unwrap_or("").to_string()
        }
        None => String::new(),
    }
}

/// Checks if a cookie domain matches a hostname.
fn domain_matches(cookie_domain: &str, hostname: &str) -> bool {
    if cookie_domain.is_empty() || hostname.is_empty() {
        return false;
    }
    if cookie_domain == hostname {
        return true;
    }
    if let Some(bare) = cookie_domain.strip_prefix('.') {
        return hostname == bare || hostname.ends_with(cookie_domain);
    }
    hostname == cookie_domain || hostname.ends_with(&format!(".{cookie_domain}"))
}

/// Checks if a cookie path matches a URL path.
fn path_matches(cookie_path: &str, url_path: &str) -> bool {
    if cookie_path.is_empty() || cookie_path == "/" {
        return true;
    }
    let url_path = if url_path.is_empty() { "/" } else { url_path };
    url_path.starts_with(cookie_path)
}

// ---------------------------------------------------------------------------
// Exported standalone cookie functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Parsed cookie information.
#[derive(Debug, Default, Clone)]
pub struct CookieInfo {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub size: i64,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: String,
}

/// Returns cookies for the given browsing context.
pub async fn get_cookies(s: &dyn Session, context: &str) -> anyhow::Result<Vec<CookieInfo>> {
    let params = json!({
        "partition": { "type": "context", "context": context },
    });

    let resp = s.send_bidi_command("storage.getCookies", params).await?;
    check_bidi_error(&resp)?;

    #[derive(Deserialize)]
    struct Inner {
        #[serde(default)]
        cookies: Vec<BidiCookie>,
    }
    #[derive(Deserialize)]
    struct Outer {
        result: Inner,
    }
    let parsed: Outer =
        serde_json::from_value(resp).map_err(|e| anyhow!("failed to parse getCookies response: {e}"))?;

    Ok(parsed
        .result
        .cookies
        .into_iter()
        .map(|c| CookieInfo {
            name: c.name,
            value: c.value.value,
            domain: c.domain,
            path: c.path,
            size: c.size,
            http_only: c.http_only,
            secure: c.secure,
            same_site: c.same_site,
        })
        .collect())
}

/// Sets a cookie in the given browsing context.
pub async fn set_cookie(
    s: &dyn Session,
    context: &str,
    name: &str,
    value: &str,
    domain: &str,
    path: &str,
) -> anyhow::Result<()> {
    // BiDi storage.setCookie requires a domain. When the caller omits it, fall
    // back to the current page's hostname (issue #152).
    let mut domain = domain.to_string();
    if domain.is_empty() {
        if let Ok(host) = eval_simple_script(s, context, "() => location.hostname").await {
            domain = host;
        }
    }

    let mut cookie_map = Map::new();
    cookie_map.insert("name".to_string(), json!(name));
    cookie_map.insert("value".to_string(), json!({ "type": "string", "value": value }));
    if !domain.is_empty() {
        cookie_map.insert("domain".to_string(), json!(domain));
    }
    if !path.is_empty() {
        cookie_map.insert("path".to_string(), json!(path));
    }

    let params = json!({
        "cookie": Value::Object(cookie_map),
        "partition": { "type": "context", "context": context },
    });

    let resp = s.send_bidi_command("storage.setCookie", params).await?;
    check_bidi_error(&resp)
}

/// Deletes cookies by name in the given browsing context. If name is empty,
/// deletes all cookies.
pub async fn delete_cookies(s: &dyn Session, context: &str, name: &str) -> anyhow::Result<()> {
    let mut params = json!({
        "partition": { "type": "context", "context": context },
    });
    if !name.is_empty() {
        params["filter"] = json!({ "name": name });
    }

    let resp = s.send_bidi_command("storage.deleteCookies", params).await?;
    check_bidi_error(&resp)
}
