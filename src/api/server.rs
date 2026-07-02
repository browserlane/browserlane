use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message as WsMessage;

use super::router::ClientTransport;

/// Maximum size of a WebSocket message (10MB).
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Boxed async callback over a client transport.
pub type ConnectFn =
    Arc<dyn Fn(Arc<dyn ClientTransport>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
/// Boxed async callback over a client transport + message.
pub type MessageFn = Arc<
    dyn Fn(Arc<dyn ClientTransport>, String) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// A WebSocket server that accepts client connections.
pub struct Server {
    host: String,
    port: u16,
    bound_port: AtomicU16,
    next_id: AtomicU64,
    on_connect: Option<ConnectFn>,
    on_message: Option<MessageFn>,
    on_close: Option<ConnectFn>,
    shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

/// Configures a Server.
pub type ServerOption = Box<dyn FnOnce(&mut Server)>;

/// Sets the bind address for the server. Every connected client gets a
/// fully-privileged browser session, so anything other than loopback exposes
/// browser control to the network.
pub fn with_host(host: &str) -> ServerOption {
    let host = host.to_string();
    Box::new(move |s| s.host = host)
}

/// Sets the port for the server.
pub fn with_port(port: u16) -> ServerOption {
    Box::new(move |s| s.port = port)
}

/// Sets a callback for when a client connects.
pub fn with_on_connect(f: ConnectFn) -> ServerOption {
    Box::new(move |s| s.on_connect = Some(f))
}

/// Sets a callback for when a message is received.
pub fn with_on_message(f: MessageFn) -> ServerOption {
    Box::new(move |s| s.on_message = Some(f))
}

/// Sets a callback for when a client disconnects.
pub fn with_on_close(f: ConnectFn) -> ServerOption {
    Box::new(move |s| s.on_close = Some(f))
}

/// Creates a new WebSocket server.
pub fn new_server(opts: Vec<ServerOption>) -> Arc<Server> {
    let mut s = Server {
        host: "127.0.0.1".to_string(),
        port: 9515,
        bound_port: AtomicU16::new(0),
        next_id: AtomicU64::new(0),
        on_connect: None,
        on_message: None,
        on_close: None,
        shutdown_tx: Mutex::new(None),
    };
    for opt in opts {
        opt(&mut s);
    }
    Arc::new(s)
}

impl Server {
    /// Returns the port the server is listening on.
    pub fn port(&self) -> u16 {
        let bound = self.bound_port.load(Ordering::SeqCst);
        if bound != 0 {
            bound
        } else {
            self.port
        }
    }

    /// Starts the WebSocket server.
    pub async fn start(self: &Arc<Self>) -> anyhow::Result<()> {
        let listener = TcpListener::bind((self.host.as_str(), self.port))
            .await
            .map_err(|e| anyhow!("failed to listen on {}:{}: {e}", self.host, self.port))?;

        let actual = listener.local_addr()?.port();
        self.bound_port.store(actual, Ordering::SeqCst);

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        *self.shutdown_tx.lock().unwrap() = Some(tx);

        let server = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut rx => return,
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _addr)) => {
                                let s = Arc::clone(&server);
                                tokio::spawn(async move { s.handle_conn(stream).await; });
                            }
                            Err(_) => continue,
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stops the WebSocket server.
    pub fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    async fn handle_conn(self: Arc<Self>, stream: TcpStream) {
        // WebSocketConfig is #[non_exhaustive]; a struct literal isn't possible.
        #[allow(clippy::field_reassign_with_default)]
        let config = {
            let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
            config.max_message_size = Some(MAX_MESSAGE_SIZE);
            config.max_frame_size = Some(MAX_MESSAGE_SIZE);
            config
        };

        let ws = match tokio_tungstenite::accept_async_with_config(stream, Some(config)).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("WebSocket upgrade error: {e}");
                return;
            }
        };

        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let (sink, mut source) = ws.split();

        // Writer task: drains the send queue to the WebSocket sink.
        let (tx, mut wrx) = tokio::sync::mpsc::channel::<String>(4096);
        tokio::spawn(async move {
            let mut sink = sink;
            while let Some(msg) = wrx.recv().await {
                if sink.send(WsMessage::Text(msg)).await.is_err() {
                    break;
                }
            }
            let _ = sink.close().await;
        });

        let client: Arc<dyn ClientTransport> = Arc::new(WsClientConn {
            id,
            tx,
            closed: AtomicBool::new(false),
        });

        eprintln!("[proxy] Client {id} connected");

        if let Some(cb) = &self.on_connect {
            cb(Arc::clone(&client)).await;
        }

        // Read loop.
        while let Some(frame) = source.next().await {
            match frame {
                Ok(WsMessage::Text(text)) => {
                    if let Some(cb) = &self.on_message {
                        cb(Arc::clone(&client), text).await;
                    }
                }
                Ok(WsMessage::Close(_)) | Err(_) => break,
                _ => {}
            }
        }

        client.close();
        eprintln!("[proxy] Client {id} disconnected");
        if let Some(cb) = &self.on_close {
            cb(Arc::clone(&client)).await;
        }
    }
}

/// A connected WebSocket client.
struct WsClientConn {
    id: u64,
    tx: tokio::sync::mpsc::Sender<String>,
    closed: AtomicBool,
}

impl ClientTransport for WsClientConn {
    fn id(&self) -> u64 {
        self.id
    }

    fn send(&self, msg: &str) -> anyhow::Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(anyhow!("connection closed"));
        }
        let _ = self.tx.try_send(msg.to_string());
        Ok(())
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}
