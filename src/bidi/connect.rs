use std::sync::Arc;

use serde_json::json;
use tokio_tungstenite::tungstenite::http::HeaderMap;

use super::connection::{connect_with_headers, Connection};
use super::session::Client;

/// Connects to a remote BiDi endpoint, creates a client, and establishes a
/// session. Returns the connection, client, and session ID.
pub async fn connect_remote(
    url: &str,
    headers: Option<HeaderMap>,
) -> anyhow::Result<(Arc<Connection>, Client, String)> {
    let conn = Arc::new(connect_with_headers(url, headers).await?);

    let client = Client::new(Arc::clone(&conn));

    match client.session_new(json!({})).await {
        Ok(result) => Ok((conn, client, result.session_id)),
        Err(e) => {
            let _ = conn.close().await;
            Err(e)
        }
    }
}
