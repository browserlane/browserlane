use anyhow::anyhow;
use serde_json::{json, Value};

use super::session::Client;

impl Client {
    /// Evaluates a JavaScript expression and returns the remote value's `value`.
    /// If context is empty, it uses the first available context.
    pub async fn evaluate(&self, context: &str, expression: &str) -> anyhow::Result<Value> {
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
            "expression": expression,
            "target": { "context": context },
            "awaitPromise": true,
            "resultOwnership": "none",
        });

        let msg = self.send_command("script.evaluate", params).await?;
        let result = msg.result.unwrap_or(Value::Null);

        let eval_type = result.get("type").and_then(Value::as_str).unwrap_or("");
        if eval_type == "exception" {
            let detail = result.get("result").cloned().unwrap_or(Value::Null);
            return Err(anyhow!("script exception: {detail}"));
        }

        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    /// Calls a JavaScript function with arguments. If context is empty, it uses
    /// the first available context. Returns the remote value's `value`.
    pub async fn call_function(
        &self,
        context: &str,
        function_declaration: &str,
        args: Vec<Value>,
    ) -> anyhow::Result<Value> {
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

        let serialized: Vec<Value> = args.iter().map(serialize_value).collect();

        let params = json!({
            "functionDeclaration": function_declaration,
            "target": { "context": context },
            "arguments": serialized,
            "awaitPromise": true,
            "resultOwnership": "none",
        });

        let msg = self.send_command("script.callFunction", params).await?;
        let result = msg.result.unwrap_or(Value::Null);

        let call_type = result.get("type").and_then(Value::as_str).unwrap_or("");
        if call_type == "exception" {
            let detail = result.get("result").cloned().unwrap_or(Value::Null);
            return Err(anyhow!("script exception: {detail}"));
        }

        // Parse the remote value's "value" field.
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }
}

/// Converts a JSON value to a BiDi serialized argument value.
fn serialize_value(v: &Value) -> Value {
    match v {
        Value::Null => json!({ "type": "undefined" }),
        Value::Bool(b) => json!({ "type": "boolean", "value": b }),
        Value::Number(n) => json!({ "type": "number", "value": n }),
        Value::String(s) => json!({ "type": "string", "value": s }),
        other => json!({ "type": "string", "value": other.to_string() }),
    }
}
