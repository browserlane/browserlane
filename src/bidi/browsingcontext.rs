use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

use super::session::Client;

/// Deserializes a string field, treating JSON `null` as "" (Go's json.Unmarshal
/// leaves the zero value on null; serde would otherwise reject it).
fn de_str_or_null<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

/// Deserializes a children array, treating JSON `null` as an empty vec.
fn de_children_or_null<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<BrowsingContextInfo>, D::Error> {
    Ok(Option::<Vec<BrowsingContextInfo>>::deserialize(d)?.unwrap_or_default())
}

/// A browsing context in the tree.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BrowsingContextInfo {
    #[serde(default, deserialize_with = "de_str_or_null")]
    pub context: String,
    #[serde(default, deserialize_with = "de_str_or_null")]
    pub url: String,
    #[serde(
        default,
        deserialize_with = "de_children_or_null",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub children: Vec<BrowsingContextInfo>,
    #[serde(
        default,
        deserialize_with = "de_str_or_null",
        skip_serializing_if = "String::is_empty"
    )]
    pub parent: String,
}

/// Result of browsingContext.getTree.
#[derive(Debug, Default, Deserialize)]
pub struct GetTreeResult {
    #[serde(default)]
    pub contexts: Vec<BrowsingContextInfo>,
}

/// Result of browsingContext.navigate.
#[derive(Debug, Default, Deserialize)]
pub struct NavigateResult {
    #[serde(default)]
    pub navigation: String,
    #[serde(default)]
    pub url: String,
}

/// Result of browsingContext.captureScreenshot.
#[derive(Debug, Default, Deserialize)]
pub struct CaptureScreenshotResult {
    /// Base64-encoded PNG.
    #[serde(default)]
    pub data: String,
}

impl Client {
    /// Returns the tree of browsing contexts.
    pub async fn get_tree(&self) -> anyhow::Result<GetTreeResult> {
        let msg = self.send_command("browsingContext.getTree", json!({})).await?;
        let result = msg.result.unwrap_or(Value::Null);
        serde_json::from_value(result)
            .map_err(|e| anyhow!("failed to parse browsingContext.getTree result: {e}"))
    }

    /// Navigates a browsing context to a URL. If context is empty, it uses the
    /// first available context.
    pub async fn navigate(&self, context: &str, url: &str) -> anyhow::Result<NavigateResult> {
        let context = if context.is_empty() {
            let tree = self
                .get_tree()
                .await
                .map_err(|e| anyhow!("failed to get browsing context: {e}"))?;
            if tree.contexts.is_empty() {
                return Err(anyhow!("no browsing contexts available"));
            }
            tree.contexts[0].context.clone()
        } else {
            context.to_string()
        };

        let params = json!({
            "context": context,
            "url": url,
            "wait": "complete",
        });

        let msg = self.send_command("browsingContext.navigate", params).await?;
        let result = msg.result.unwrap_or(Value::Null);
        serde_json::from_value(result)
            .map_err(|e| anyhow!("failed to parse browsingContext.navigate result: {e}"))
    }

    /// Returns the URL of the first browsing context.
    pub async fn get_current_url(&self) -> anyhow::Result<String> {
        let tree = self.get_tree().await?;
        if tree.contexts.is_empty() {
            return Err(anyhow!("no browsing contexts available"));
        }
        Ok(tree.contexts[0].url.clone())
    }

    /// Captures a screenshot of the viewport. If context is empty, it uses the
    /// first available context. Returns base64-encoded PNG data.
    pub async fn capture_screenshot(&self, context: &str) -> anyhow::Result<String> {
        let context = self.resolve_first_context(context).await?;
        let params = json!({ "context": context });
        let msg = self
            .send_command("browsingContext.captureScreenshot", params)
            .await?;
        let result: CaptureScreenshotResult =
            serde_json::from_value(msg.result.unwrap_or(Value::Null))
                .map_err(|e| anyhow!("failed to parse browsingContext.captureScreenshot result: {e}"))?;
        Ok(result.data)
    }

    /// Captures a full-page screenshot (entire document, not just viewport).
    pub async fn capture_full_page_screenshot(&self, context: &str) -> anyhow::Result<String> {
        let context = self.resolve_first_context(context).await?;
        let params = json!({ "context": context, "origin": "document" });
        let msg = self
            .send_command("browsingContext.captureScreenshot", params)
            .await?;
        let result: CaptureScreenshotResult =
            serde_json::from_value(msg.result.unwrap_or(Value::Null))
                .map_err(|e| anyhow!("failed to parse browsingContext.captureScreenshot result: {e}"))?;
        Ok(result.data)
    }

    pub(super) async fn resolve_first_context(&self, context: &str) -> anyhow::Result<String> {
        if !context.is_empty() {
            return Ok(context.to_string());
        }
        let tree = self
            .get_tree()
            .await
            .map_err(|e| anyhow!("failed to get browsing context: {e}"))?;
        if tree.contexts.is_empty() {
            return Err(anyhow!("no browsing contexts available"));
        }
        Ok(tree.contexts[0].context.clone())
    }
}
