use std::time::Duration;

use anyhow::anyhow;
use serde_json::{Map, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::dial;
use super::router::StatusResult;
use crate::agent;
use crate::paths;

const DIAL_TIMEOUT: Duration = Duration::from_secs(2);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Sends a tools/call request to the daemon and returns the result.
pub async fn call(tool_name: &str, args: Map<String, Value>) -> anyhow::Result<agent::ToolsCallResult> {
    let params = agent::ToolsCallParams {
        name: tool_name.to_string(),
        arguments: args,
    };
    let params_json =
        serde_json::to_value(&params).map_err(|e| anyhow!("marshal params: {e}"))?;

    let resp = send_request("tools/call", Some(params_json)).await?;

    if let Some(err) = resp.error {
        return Err(anyhow!("daemon error: {}", err.message));
    }

    let result: agent::ToolsCallResult =
        serde_json::from_value(resp.result.unwrap_or(Value::Null))
            .map_err(|e| anyhow!("unmarshal result: {e}"))?;

    if result.is_error {
        if let Some(first) = result.content.first() {
            return Err(anyhow!("{}", first.text));
        }
        return Err(anyhow!("tool call failed"));
    }

    Ok(result)
}

/// Sends a daemon/status request and returns the result.
pub async fn status() -> anyhow::Result<StatusResult> {
    let resp = send_request("daemon/status", None).await?;
    if let Some(err) = resp.error {
        return Err(anyhow!("daemon error: {}", err.message));
    }
    serde_json::from_value(resp.result.unwrap_or(Value::Null))
        .map_err(|e| anyhow!("unmarshal result: {e}"))
}

/// Sends a daemon/shutdown request.
pub async fn shutdown() -> anyhow::Result<()> {
    let resp = send_request("daemon/shutdown", None).await?;
    if let Some(err) = resp.error {
        return Err(anyhow!("daemon error: {}", err.message));
    }
    Ok(())
}

/// Sends a JSON-RPC request to the daemon socket and returns the response.
async fn send_request(method: &str, params: Option<Value>) -> anyhow::Result<agent::Response> {
    let socket_path = paths::get_socket_path()
        .map_err(|e| anyhow!("get socket path: {e}"))?
        .to_string_lossy()
        .into_owned();

    let conn = dial(&socket_path, DIAL_TIMEOUT)
        .await
        .map_err(|e| anyhow!("connect to daemon: {e}"))?;

    let req = agent::Request {
        jsonrpc: "2.0".to_string(),
        id: Some(Value::from(1)),
        method: method.to_string(),
        params,
    };

    let data = serde_json::to_string(&req).map_err(|e| anyhow!("marshal request: {e}"))?;

    let (read_half, mut write_half) = tokio::io::split(conn);

    tokio::time::timeout(Duration::from_secs(5), async {
        write_half.write_all(data.as_bytes()).await?;
        write_half.write_all(b"\n").await?;
        write_half.flush().await
    })
    .await
    .map_err(|_| anyhow!("write request: timeout"))?
    .map_err(|e| anyhow!("write request: {e}"))?;

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    let n = tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut line))
        .await
        .map_err(|_| anyhow!("read response: timeout"))?
        .map_err(|e| anyhow!("read response: {e}"))?;
    if n == 0 {
        return Err(anyhow!("daemon closed connection without response"));
    }

    serde_json::from_str(line.trim_end()).map_err(|e| anyhow!("unmarshal response: {e}"))
}
