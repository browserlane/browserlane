use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use anyhow::anyhow;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderMap;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async_with_config, MaybeTlsStream, WebSocketStream};

use crate::errors::ConnectionError;

/// Maximum size of a WebSocket message (10MB). Accommodates large screenshots
/// from high-resolution displays (e.g. retina, 4K).
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Timeout for each WebSocket read operation. Must be longer than
/// `PING_INTERVAL` so pongs have time to arrive.
const READ_DEADLINE: Duration = Duration::from_secs(120);

/// How often we send WebSocket pings to keep the connection alive.
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// Handshake timeout for establishing the WebSocket connection.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, WsMessage>;
type WsSource = SplitStream<WsStream>;

/// A WebSocket connection to the browser.
pub struct Connection {
    write: Arc<TokioMutex<WsSink>>,
    read: TokioMutex<WsSource>,
    closed: Arc<AtomicBool>,
    ping_handle: StdMutex<Option<tokio::task::JoinHandle<()>>>,
}

/// Establishes a WebSocket connection to the given URL.
pub async fn connect(url: &str) -> anyhow::Result<Connection> {
    connect_with_headers(url, None).await
}

/// Establishes a WebSocket connection with optional HTTP headers. Headers are
/// sent during the WebSocket handshake (useful for authentication tokens).
pub async fn connect_with_headers(
    url: &str,
    headers: Option<HeaderMap>,
) -> anyhow::Result<Connection> {
    let mut request = url.into_client_request().map_err(|e| ConnectionError {
        url: url.to_string(),
        cause: Some(Box::new(e)),
    })?;
    if let Some(h) = headers {
        request.headers_mut().extend(h);
    }

    // WebSocketConfig is #[non_exhaustive], so a struct literal isn't possible
    // from outside its crate; mutate the relevant fields on a Default instance.
    #[allow(clippy::field_reassign_with_default)]
    let config = {
        let mut config = WebSocketConfig::default();
        config.max_message_size = Some(MAX_MESSAGE_SIZE);
        config.max_frame_size = Some(MAX_MESSAGE_SIZE);
        config
    };

    let dial = connect_async_with_config(request, Some(config), false);
    let (ws, _resp) = match timeout(HANDSHAKE_TIMEOUT, dial).await {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => {
            return Err(ConnectionError {
                url: url.to_string(),
                cause: Some(Box::new(e)),
            }
            .into())
        }
        Err(_) => {
            return Err(ConnectionError {
                url: url.to_string(),
                cause: Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "handshake timeout",
                ))),
            }
            .into())
        }
    };

    let (sink, source) = ws.split();
    let write = Arc::new(TokioMutex::new(sink));
    let closed = Arc::new(AtomicBool::new(false));

    let ping_write = Arc::clone(&write);
    let ping_closed = Arc::clone(&closed);
    let ping_handle = tokio::spawn(async move { ping_loop(ping_write, ping_closed).await });

    Ok(Connection {
        write,
        read: TokioMutex::new(source),
        closed,
        ping_handle: StdMutex::new(Some(ping_handle)),
    })
}

/// Sends WebSocket pings at regular intervals to keep the connection alive.
async fn ping_loop(write: Arc<TokioMutex<WsSink>>, closed: Arc<AtomicBool>) {
    let start = tokio::time::Instant::now() + PING_INTERVAL;
    let mut interval = tokio::time::interval_at(start, PING_INTERVAL);
    loop {
        interval.tick().await;
        if closed.load(Ordering::SeqCst) {
            return;
        }
        let mut w = write.lock().await;
        if w.send(WsMessage::Ping(Vec::new())).await.is_err() {
            return;
        }
    }
}

impl Connection {
    /// Sends a text message over the WebSocket.
    pub async fn send(&self, msg: &str) -> anyhow::Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(anyhow!("connection closed"));
        }
        let mut w = self.write.lock().await;
        w.send(WsMessage::Text(msg.to_string()))
            .await
            .map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// Receives a text message from the WebSocket. Blocks until a data message
    /// arrives or the read deadline (120s) expires. Control frames (ping/pong)
    /// are handled transparently, mirroring gorilla's `ReadMessage`.
    pub async fn receive(&self) -> anyhow::Result<String> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(anyhow!("connection closed"));
        }

        let mut read = self.read.lock().await;
        loop {
            let frame = match timeout(READ_DEADLINE, read.next()).await {
                Err(_) => return Err(anyhow!("read deadline exceeded")),
                Ok(None) => return Err(anyhow!("connection closed")),
                Ok(Some(Ok(frame))) => frame,
                Ok(Some(Err(e))) => return Err(anyhow!(e)),
            };

            match frame {
                WsMessage::Text(text) => return Ok(text),
                WsMessage::Binary(_) => {
                    return Err(anyhow!("expected text message, got type 2"))
                }
                WsMessage::Ping(payload) => {
                    let mut w = self.write.lock().await;
                    let _ = w.send(WsMessage::Pong(payload)).await;
                }
                WsMessage::Pong(_) | WsMessage::Frame(_) => {}
                WsMessage::Close(_) => return Err(anyhow!("connection closed")),
            }
        }
    }

    /// Closes the WebSocket connection.
    pub async fn close(&self) -> anyhow::Result<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        if let Some(handle) = self.ping_handle.lock().unwrap().take() {
            handle.abort();
        }

        let mut w = self.write.lock().await;
        let _ = w
            .send(WsMessage::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "".into(),
            })))
            .await;
        let _ = w.close().await;
        Ok(())
    }
}
