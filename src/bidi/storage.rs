use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

use super::session::Client;

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(f: &f64) -> bool {
    *f == 0.0
}

/// Accepts the cookie value either as a plain string or as a BiDi typed value
/// object (`{"type":"string","value":"..."}`). storage.getCookies returns the
/// latter, which would otherwise fail to deserialize into the string field
/// (issue #150).
fn de_cookie_value<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum CookieValue {
        Str(String),
        Typed {
            #[serde(default)]
            value: String,
        },
    }
    match Option::<CookieValue>::deserialize(d)? {
        Some(CookieValue::Str(s)) => Ok(s),
        Some(CookieValue::Typed { value }) => Ok(value),
        None => Ok(String::new()),
    }
}

/// A browser cookie.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    #[serde(default, deserialize_with = "de_cookie_value")]
    pub value: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub domain: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secure: bool,
    #[serde(rename = "httpOnly", default, skip_serializing_if = "is_false")]
    pub http_only: bool,
    #[serde(rename = "sameSite", default, skip_serializing_if = "String::is_empty")]
    pub same_site: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub size: f64,
}

impl Client {
    /// Returns all cookies for the given browsing context. If context is empty,
    /// it uses the first available context.
    pub async fn get_cookies(&self, context: &str) -> anyhow::Result<Vec<Cookie>> {
        let context = self.resolve_first_context(context).await?;

        let params = json!({
            "partition": {
                "type": "context",
                "context": context,
            },
        });

        let msg = self.send_command("storage.getCookies", params).await?;
        let result = msg.result.unwrap_or(Value::Null);

        #[derive(Deserialize)]
        struct GetCookiesResult {
            #[serde(default)]
            cookies: Vec<Cookie>,
        }
        let parsed: GetCookiesResult = serde_json::from_value(result)
            .map_err(|e| anyhow!("failed to parse storage.getCookies result: {e}"))?;
        Ok(parsed.cookies)
    }

    /// Sets a cookie in the given browsing context. If context is empty, it uses
    /// the first available context.
    pub async fn set_cookie(&self, context: &str, cookie: Cookie) -> anyhow::Result<()> {
        let context = self.resolve_first_context(context).await?;

        let mut cookie_map = serde_json::Map::new();
        cookie_map.insert("name".to_string(), json!(cookie.name));
        cookie_map.insert(
            "value".to_string(),
            json!({ "type": "string", "value": cookie.value }),
        );
        if !cookie.domain.is_empty() {
            cookie_map.insert("domain".to_string(), json!(cookie.domain));
        }
        if !cookie.path.is_empty() {
            cookie_map.insert("path".to_string(), json!(cookie.path));
        }

        let params = json!({
            "cookie": Value::Object(cookie_map),
            "partition": {
                "type": "context",
                "context": context,
            },
        });

        self.send_command("storage.setCookie", params).await?;
        Ok(())
    }
}
