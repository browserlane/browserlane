use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use anyhow::anyhow;
use tokio::sync::Mutex as TokioMutex;
use tokio_tungstenite::tungstenite::http::HeaderMap;

use crate::agent;
use crate::log;
use crate::paths;

use super::pidfile::{remove_pid, write_pid};

#[cfg(unix)]
use super::listener_unix::listen;
#[cfg(windows)]
use super::listener_windows::listen;
// Brings `Listener::accept` into scope (the value's concrete type is
// interprocess's tokio Listener; the method lives on this trait).
use interprocess::local_socket::traits::tokio::Listener as _;

/// Configures a new Daemon.
pub struct Options {
    pub version: String,
    pub screenshot_dir: String,
    pub headless: bool,
    pub idle_timeout: Duration,
    /// Remote BiDi WebSocket URL (empty = local browser).
    pub connect_url: String,
    /// Headers for remote WebSocket connection.
    pub connect_headers: Option<HeaderMap>,
}

/// Manages a long-lived browser session accessible via a Unix socket.
pub struct Daemon {
    pub(crate) handlers: TokioMutex<agent::Handlers>,
    pub(crate) version: String,
    pub(crate) start_time: Instant,
    pub(crate) start_time_sys: SystemTime,
    pub(crate) last_activity: Mutex<Instant>,
    idle_timeout: Duration,
    pub(crate) socket_path: Mutex<String>,
    shutdown_notify: tokio::sync::Notify,
    shutdown_started: AtomicBool,
}

/// Creates a new Daemon instance.
pub fn new(opts: Options) -> Arc<Daemon> {
    Arc::new(Daemon {
        handlers: TokioMutex::new(agent::new_handlers(
            &opts.screenshot_dir,
            opts.headless,
            &opts.connect_url,
            opts.connect_headers,
        )),
        version: opts.version,
        start_time: Instant::now(),
        start_time_sys: SystemTime::now(),
        last_activity: Mutex::new(Instant::now()),
        idle_timeout: opts.idle_timeout,
        socket_path: Mutex::new(String::new()),
        shutdown_notify: tokio::sync::Notify::new(),
        shutdown_started: AtomicBool::new(false),
    })
}

impl Daemon {
    /// Starts the daemon, listening for connections until shutdown.
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        let socket_path = paths::get_socket_path()
            .map_err(|e| anyhow!("get socket path: {e}"))?
            .to_string_lossy()
            .into_owned();
        *self.socket_path.lock().unwrap() = socket_path.clone();

        let dir = paths::get_daemon_dir().map_err(|e| anyhow!("get daemon dir: {e}"))?;
        std::fs::create_dir_all(&dir).map_err(|e| anyhow!("create daemon dir: {e}"))?;

        // Remove stale socket file.
        let _ = std::fs::remove_file(&socket_path);

        let listener = listen(&socket_path).map_err(|e| anyhow!("listen: {e}"))?;

        if let Err(e) = write_pid() {
            drop(listener);
            return Err(anyhow!("write PID: {e}"));
        }

        log::debug("daemon started");

        if self.idle_timeout > Duration::ZERO {
            let d = Arc::clone(&self);
            tokio::spawn(async move { d.watch_idle().await });
        }

        loop {
            tokio::select! {
                _ = self.shutdown_notify.notified() => break,
                accepted = listener.accept() => {
                    match accepted {
                        Ok(conn) => {
                            let d = Arc::clone(&self);
                            tokio::spawn(async move { d.handle_connection(conn).await });
                        }
                        Err(e) => {
                            log::debug(&format!("accept error: {e}"));
                            continue;
                        }
                    }
                }
            }
        }

        self.do_shutdown().await;
        Ok(())
    }

    /// Triggers a clean daemon shutdown (idempotent).
    pub fn shutdown(&self) {
        self.shutdown_notify.notify_one();
    }

    /// Performs the actual shutdown cleanup exactly once.
    async fn do_shutdown(&self) {
        if self.shutdown_started.swap(true, Ordering::SeqCst) {
            return;
        }
        log::debug("daemon shutting down");

        // Close the browser session.
        self.handlers.lock().await.close().await;

        // Clean up socket file.
        let socket_path = self.socket_path.lock().unwrap().clone();
        if !socket_path.is_empty() {
            let _ = std::fs::remove_file(&socket_path);
        }

        // Remove PID file.
        let _ = remove_pid();
    }

    /// Updates the last activity timestamp.
    pub(crate) fn touch_activity(&self) {
        *self.last_activity.lock().unwrap() = Instant::now();
    }

    /// Monitors for idle timeout and triggers shutdown.
    async fn watch_idle(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.tick().await; // consume the immediate first tick

        loop {
            interval.tick().await;
            let idle = self.last_activity.lock().unwrap().elapsed();
            if idle >= self.idle_timeout {
                log::debug("idle timeout reached, shutting down");
                self.shutdown();
                return;
            }
        }
    }
}
