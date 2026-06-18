//! Phase 2 ports the dispatch FRAMEWORK (client lifecycle, browser<->client
//! routing, internal command tracking) plus the `browserlane:page.navigate` route.
//! All other `browserlane:` routes fall through to the faithful default: forward the
//! raw message to the browser (the same path Go uses for unknown commands).
//! The remaining route handlers are ported in Phase 3.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::http::HeaderMap;

use super::handlers_recording::capture_recording_screenshot;
use super::helpers::BoxInfo;
use super::recording::{now_unix_millis, Recorder};
use super::session::new_api_session;
use crate::bidi;
use crate::browser::{self, LaunchOptions, LaunchResult};

/// Default timeout for element resolution and actionability checks.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Transport implemented by both the WebSocket server and the stdio pipe.
pub trait ClientTransport: Send + Sync {
    fn id(&self) -> u64;
    fn send(&self, msg: &str) -> anyhow::Result<()>;
    fn close(&self);
}

/// A BiDi command parsed from an incoming client message.
#[derive(Debug, Default, Deserialize)]
pub struct BidiCommand {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub params: Map<String, Value>,
}

/// A BiDi response sent to the client (field order mirrors Go's bidiResponse).
#[derive(Debug, Serialize)]
struct BidiResponse {
    id: i64,
    #[serde(rename = "type")]
    response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "String::is_empty")]
    error: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    message: String,
}

/// A browser session connected to a client.
pub struct BrowserSession {
    pub launch_result: Option<Arc<LaunchResult>>,
    pub bidi_conn: Arc<bidi::Connection>,
    pub bidi_client: Option<Arc<bidi::Client>>,
    pub client: Arc<dyn ClientTransport>,
    pub(crate) closed: AtomicBool,
    internal_cmds: Mutex<HashMap<i64, oneshot::Sender<Value>>>,
    next_internal_id: AtomicI64,
    pub(crate) last_context: Mutex<String>,
    pub(crate) last_url: Mutex<String>,
    last_element_box: Mutex<Option<BoxInfo>>,
    pub(crate) dispatch_mu: tokio::sync::Mutex<()>,
    /// IDs of client commands currently being handled (one terminal reply each).
    /// On connection close, any still-pending id gets a "connection closed" error
    /// reply — matching Go, where the in-flight handler's send hits the closed
    /// connection and surfaces that error to the client.
    in_flight: Mutex<HashSet<i64>>,
    /// Active recorder (Go's `session.recorder`); None when not recording.
    pub(crate) recorder: Mutex<Option<Arc<Recorder>>>,
    /// Atomic; true while a per-action filmstrip screenshot capture is in progress
    /// (Go's `screenshotInFlight`).
    pub(crate) screenshot_in_flight: AtomicBool,
    /// Atomic; true when a handler already captured a filmstrip screenshot for the
    /// current action (Go's `handlerScreenshot`), so dispatch() skips its own.
    pub(crate) handler_screenshot: AtomicBool,
    /// Preload-script ID for the installed fake clock; empty until clock.install
    /// registers it (so we only register the preload script once per session).
    pub(crate) clock_preload_script_id: Mutex<String>,
    /// Preload-script ID for the WebSocket monitor; empty until page.onWebSocket
    /// installs it (so we only register the preload script once per session).
    pub(crate) ws_preload_script_id: Mutex<String>,
    /// Whether `script.message` is subscribed (for the WS monitor channel).
    pub(crate) ws_subscribed: AtomicBool,
    /// Temp dir downloads are saved into; cleaned up on session close.
    pub(crate) download_dir: Mutex<String>,
}

impl BrowserSession {
    /// Stores the bounding box of the last resolved element (for recording).
    pub fn set_last_element_box(&self, box_: BoxInfo) {
        *self.last_element_box.lock().unwrap() = Some(box_);
    }
}

/// Manages browser sessions for connected clients.
pub struct Router {
    sessions: Mutex<HashMap<u64, Arc<BrowserSession>>>,
    headless: bool,
    connect_url: String,
    connect_headers: Option<HeaderMap>,
}

/// Creates a new router.
pub fn new_router(headless: bool, connect_url: &str, connect_headers: Option<HeaderMap>) -> Arc<Router> {
    Arc::new(Router {
        sessions: Mutex::new(HashMap::new()),
        headless,
        connect_url: connect_url.to_string(),
        connect_headers,
    })
}

impl Router {
    /// Called when a new client connects. Launches a browser (or connects to a
    /// remote one), establishes a BiDi connection, and subscribes to events.
    pub async fn on_client_connect(self: &Arc<Self>, client: Arc<dyn ClientTransport>) {
        let launch_result: Option<Arc<LaunchResult>>;
        let bidi_conn: Arc<bidi::Connection>;
        let bidi_client: Option<Arc<bidi::Client>>;

        if !self.connect_url.is_empty() {
            // Remote mode: connect to an existing BiDi endpoint.
            eprintln!(
                "[router] Connecting to remote browser for client {}: {}",
                client.id(),
                self.connect_url
            );

            match bidi::connect_remote(&self.connect_url, self.connect_headers.clone()).await {
                Ok((conn, c, _session_id)) => {
                    eprintln!(
                        "[router] Remote BiDi connection established for client {}",
                        client.id()
                    );
                    bidi_conn = conn;
                    bidi_client = Some(Arc::new(c));
                    launch_result = None;
                }
                Err(e) => {
                    eprintln!(
                        "[router] Failed to connect to remote browser for client {}: {}",
                        client.id(),
                        e
                    );
                    let _ = client.send(&format!(
                        r#"{{"error":{{"code":-32000,"message":"Failed to connect to remote browser: {e}"}}}}"#
                    ));
                    client.close();
                    return;
                }
            }
        } else {
            // Local mode: launch a browser.
            eprintln!("[router] Launching browser for client {}...", client.id());

            let lr = match browser::launch(LaunchOptions {
                headless: self.headless,
                port: 0,
                verbose: false,
            })
            .await
            {
                Ok(lr) => lr,
                Err(e) => {
                    eprintln!(
                        "[router] Failed to launch browser for client {}: {}",
                        client.id(),
                        e
                    );
                    let _ = client.send(&format!(
                        r#"{{"error":{{"code":-32000,"message":"Failed to launch browser: {e}"}}}}"#
                    ));
                    client.close();
                    return;
                }
            };

            if let Some(conn) = lr.bidi_conn.clone() {
                eprintln!(
                    "[router] Browser launched for client {} (BiDi session)",
                    client.id()
                );
                bidi_conn = conn;
            } else {
                eprintln!(
                    "[router] Browser launched for client {}, WebSocket: {}",
                    client.id(),
                    lr.web_socket_url
                );
                match bidi::connect(&lr.web_socket_url).await {
                    Ok(c) => {
                        eprintln!(
                            "[router] BiDi connection established for client {}",
                            client.id()
                        );
                        bidi_conn = Arc::new(c);
                    }
                    Err(e) => {
                        eprintln!(
                            "[router] Failed to connect to browser BiDi for client {}: {}",
                            client.id(),
                            e
                        );
                        let _ = lr.close().await;
                        let _ = client.send(&format!(
                            r#"{{"error":{{"code":-32000,"message":"Failed to connect to browser: {e}"}}}}"#
                        ));
                        client.close();
                        return;
                    }
                }
            }

            bidi_client = Some(Arc::new(bidi::Client::new(Arc::clone(&bidi_conn))));
            launch_result = Some(Arc::new(lr));
        }

        let session = Arc::new(BrowserSession {
            launch_result,
            bidi_conn,
            bidi_client,
            client: Arc::clone(&client),
            closed: AtomicBool::new(false),
            internal_cmds: Mutex::new(HashMap::new()),
            next_internal_id: AtomicI64::new(1_000_000),
            last_context: Mutex::new(String::new()),
            last_url: Mutex::new(String::new()),
            last_element_box: Mutex::new(None),
            dispatch_mu: tokio::sync::Mutex::new(()),
            in_flight: Mutex::new(HashSet::new()),
            recorder: Mutex::new(None),
            screenshot_in_flight: AtomicBool::new(false),
            handler_screenshot: AtomicBool::new(false),
            clock_preload_script_id: Mutex::new(String::new()),
            ws_preload_script_id: Mutex::new(String::new()),
            ws_subscribed: AtomicBool::new(false),
            download_dir: Mutex::new(String::new()),
        });

        self.sessions.lock().unwrap().insert(client.id(), Arc::clone(&session));

        // Start routing messages from browser to client.
        let router = Arc::clone(self);
        let route_session = Arc::clone(&session);
        tokio::spawn(async move { router.route_browser_to_client(route_session).await });

        // Subscribe to events synchronously — must complete before client
        // commands so Chrome delivers events from the first navigation.
        let subscribe = json!({
            "events": [
                "browsingContext.contextCreated",
                "network.beforeRequestSent",
                "network.responseCompleted",
                "browsingContext.userPromptOpened",
                "log.entryAdded",
                "browsingContext.downloadWillBegin",
                "browsingContext.downloadEnd",
                "browsingContext.load",
                "browsingContext.fragmentNavigated",
            ],
        });
        if let Err(e) = self
            .send_internal_command(&session, "session.subscribe", subscribe)
            .await
        {
            eprintln!(
                "[router] Failed to subscribe to events for client {}: {}",
                client.id(),
                e
            );
        }

        // Download setup is non-critical — run in background so it doesn't block
        // client commands if Chrome is slow to respond.
        let dl_router = Arc::clone(self);
        let dl_session = Arc::clone(&session);
        tokio::spawn(async move { dl_router.setup_downloads(dl_session).await });
    }

    /// Called when a message is received from a client. Handles custom `browserlane:`
    /// commands or forwards to the browser.
    pub async fn on_client_message(self: &Arc<Self>, client: &Arc<dyn ClientTransport>, msg: String) {
        let session = match self.sessions.lock().unwrap().get(&client.id()).cloned() {
            Some(s) => s,
            None => {
                eprintln!("[router] No session for client {}", client.id());
                return;
            }
        };

        if session.closed.load(Ordering::SeqCst) {
            return;
        }

        // Parse the command to check for custom browserlane: extension methods.
        let cmd: BidiCommand = match serde_json::from_str(&msg) {
            Ok(c) => c,
            Err(_) => {
                if let Err(e) = session.bidi_conn.send(&msg).await {
                    eprintln!(
                        "[router] Failed to send to browser for client {}: {}",
                        client.id(),
                        e
                    );
                }
                return;
            }
        };

        match cmd.method.as_str() {
            "browserlane:page.navigate" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_navigate(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.back" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_back(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.forward" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_forward(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.reload" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_reload(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.url" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_url(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.title" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_title(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.content" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_content(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.waitForURL" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_wait_for_url(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.waitForLoad" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_wait_for_load(&session, cmd).await })
                });
                return;
            }
            "browserlane:browser.page" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browser_page(&session, cmd).await })
                });
                return;
            }
            "browserlane:browser.pages" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browser_pages(&session, cmd).await })
                });
                return;
            }
            "browserlane:browser.newPage" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browser_new_page(&session, cmd).await })
                });
                return;
            }
            "browserlane:browser.newContext" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browser_new_context(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.newPage" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_new_page(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.close" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_close(&session, cmd).await })
                });
                return;
            }
            "browserlane:browser.stop" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browser_stop(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.activate" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_activate(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.close" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_close(&session, cmd).await })
                });
                return;
            }
            "browserlane:keyboard.press" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_keyboard_press(&session, cmd).await })
                });
                return;
            }
            "browserlane:keyboard.down" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_keyboard_down(&session, cmd).await })
                });
                return;
            }
            "browserlane:keyboard.up" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_keyboard_up(&session, cmd).await })
                });
                return;
            }
            "browserlane:keyboard.type" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_keyboard_type(&session, cmd).await })
                });
                return;
            }
            "browserlane:mouse.click" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_mouse_click(&session, cmd).await })
                });
                return;
            }
            "browserlane:mouse.move" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_mouse_move(&session, cmd).await })
                });
                return;
            }
            "browserlane:mouse.down" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_mouse_down(&session, cmd).await })
                });
                return;
            }
            "browserlane:mouse.up" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_mouse_up(&session, cmd).await })
                });
                return;
            }
            "browserlane:mouse.wheel" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_mouse_wheel(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.scroll" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_scroll(&session, cmd).await })
                });
                return;
            }
            "browserlane:touch.tap" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_touch_tap(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.screenshot" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_screenshot(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.pdf" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_pdf(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.screenshot" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_screenshot(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.bounds" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_bounds(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.highlight" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_highlight(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.waitFor" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_wait_for(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.waitFor" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_wait_for(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.wait" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_wait(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.waitForFunction" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_wait_for_function(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.eval" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_eval(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.addScript" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_add_script(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.addStyle" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_add_style(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.expose" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_expose(&session, cmd).await })
                });
                return;
            }
            // --- Storage cluster (cookies / localStorage / init scripts) ---
            "browserlane:context.cookies" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_cookies(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.setCookies" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_set_cookies(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.clearCookies" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_clear_cookies(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.storage" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_storage(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.setStorage" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_set_storage(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.clearStorage" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_clear_storage(&session, cmd).await })
                });
                return;
            }
            "browserlane:context.addInitScript" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_context_add_init_script(&session, cmd).await })
                });
                return;
            }
            // --- Dialog cluster (alert / confirm / prompt) ---
            "browserlane:dialog.accept" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_dialog_accept(&session, cmd).await })
                });
                return;
            }
            "browserlane:dialog.dismiss" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_dialog_dismiss(&session, cmd).await })
                });
                return;
            }
            // --- WebSocket monitoring ---
            "browserlane:page.onWebSocket" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_on_web_socket(&session, cmd).await })
                });
                return;
            }
            // --- Network cluster (request interception) ---
            "browserlane:page.route" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_route(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.unroute" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_unroute(&session, cmd).await })
                });
                return;
            }
            // continue/fulfill/abort run concurrently (no dispatch_mu) — they must
            // resolve a request while another command (a blocked eval) holds the lock.
            "browserlane:network.continue" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_network_continue(&session, cmd).await })
                });
                return;
            }
            "browserlane:network.fulfill" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_network_fulfill(&session, cmd).await })
                });
                return;
            }
            "browserlane:network.abort" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_network_abort(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.setHeaders" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_set_headers(&session, cmd).await })
                });
                return;
            }
            // --- Clock cluster (fake timers) ---
            "browserlane:clock.install" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_install(&session, cmd).await })
                });
                return;
            }
            "browserlane:clock.fastForward" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_fast_forward(&session, cmd).await })
                });
                return;
            }
            "browserlane:clock.runFor" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_run_for(&session, cmd).await })
                });
                return;
            }
            "browserlane:clock.pauseAt" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_pause_at(&session, cmd).await })
                });
                return;
            }
            "browserlane:clock.resume" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_resume(&session, cmd).await })
                });
                return;
            }
            "browserlane:clock.setFixedTime" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_set_fixed_time(&session, cmd).await })
                });
                return;
            }
            "browserlane:clock.setSystemTime" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_set_system_time(&session, cmd).await })
                });
                return;
            }
            "browserlane:clock.setTimezone" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_clock_set_timezone(&session, cmd).await })
                });
                return;
            }
            // --- Download cluster (saveAs) ---
            "browserlane:download.saveAs" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_download_save_as(&session, cmd).await })
                });
                return;
            }
            // --- A11y cluster (role / label / a11yTree) ---
            "browserlane:element.role" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_role(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.label" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_label(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.a11yTree" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_page_a11y_tree(&session, cmd).await })
                });
                return;
            }
            // --- Frames cluster (frame / frames) ---
            "browserlane:page.frames" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_frames(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.frame" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_frame(&session, cmd).await })
                });
                return;
            }
            // --- Emulation cluster (viewport / window / media / geolocation / setContent) ---
            "browserlane:page.setViewport" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_set_viewport(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.viewport" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_viewport(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.emulateMedia" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_emulate_media(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.setContent" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_set_content(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.setGeolocation" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_set_geolocation(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.setWindow" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_set_window(&session, cmd).await })
                });
                return;
            }
            "browserlane:page.window" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_page_window(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.find" | "browserlane:page.find" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_find(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.findAll" | "browserlane:page.findAll" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_find_all(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.text" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_text(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.innerText" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_inner_text(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.html" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_html(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.value" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_value(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.attr" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_attr(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.isVisible" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_is_visible(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.isHidden" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_is_hidden(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.isEnabled" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_is_enabled(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.isChecked" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_is_checked(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.isEditable" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_is_editable(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.click" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_click(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.dblclick" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_dblclick(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.fill" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_fill(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.type" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_type(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.press" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_press(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.clear" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_clear(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.check" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_check(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.uncheck" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_uncheck(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.selectOption" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_select_option(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.hover" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_hover(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.focus" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_focus(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.dragTo" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_drag_to(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.tap" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_tap(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.scrollIntoView" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_scroll_into_view(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.dispatchEvent" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_dispatch_event(&session, cmd).await })
                });
                return;
            }
            "browserlane:element.setFiles" => {
                self.dispatch(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_browserlane_el_set_files(&session, cmd).await })
                });
                return;
            }

            // --- Recording cluster (not recorded — they control recording itself) ---
            // Dispatched concurrently (NOT under dispatch_mu): the stop/chunk/group
            // handlers acquire dispatch_mu themselves to order events, so routing
            // them through `dispatch` would deadlock. Mirrors Go's bare
            // `go r.handleRecordingX(session, cmd)`.
            "browserlane:recording.start" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_recording_start(&session, cmd).await })
                });
                return;
            }
            "browserlane:recording.stop" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_recording_stop(&session, cmd).await })
                });
                return;
            }
            "browserlane:recording.startChunk" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_recording_start_chunk(&session, cmd).await })
                });
                return;
            }
            "browserlane:recording.stopChunk" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_recording_stop_chunk(&session, cmd).await })
                });
                return;
            }
            "browserlane:recording.startGroup" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_recording_start_group(&session, cmd).await })
                });
                return;
            }
            "browserlane:recording.stopGroup" => {
                self.dispatch_concurrent(&session, cmd, |router, session, cmd| {
                    Box::pin(async move { router.handle_recording_stop_group(&session, cmd).await })
                });
                return;
            }
            _ => {}
        }

        // Forward standard BiDi commands to the browser.
        if let Err(e) = session.bidi_conn.send(&msg).await {
            eprintln!(
                "[router] Failed to send to browser for client {}: {}",
                client.id(),
                e
            );
        }
    }

    /// Wraps a route handler in a serialized background task (mirrors Go's
    /// `dispatch`, which runs handlers in a goroutine under dispatchMu) and adds
    /// automatic action recording (before/after events, snapshots, filmstrip
    /// screenshots) when a recorder is active.
    fn dispatch<F>(self: &Arc<Self>, session: &Arc<BrowserSession>, mut cmd: BidiCommand, handler: F)
    where
        F: FnOnce(
                Arc<Router>,
                Arc<BrowserSession>,
                BidiCommand,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            + Send
            + 'static,
    {
        // Register the command as in-flight so a premature connection close
        // can flush its terminal "connection closed" error reply.
        session.in_flight.lock().unwrap().insert(cmd.id);

        let router = Arc::clone(self);
        let session = Arc::clone(session);
        tokio::spawn(async move {
            let _guard = session.dispatch_mu.lock().await;

            let recorder = session.recorder.lock().unwrap().clone();
            let method = cmd.method.clone();

            let mut call_id = String::new();
            if let Some(rec) = &recorder {
                if rec.is_recording() {
                    call_id = rec.next_call_id();
                    let opts = rec.options();

                    // Interaction handlers (click, fill, etc.) capture the
                    // before-snapshot inside the handler after scrolling the
                    // element into view, so the screenshot matches the element
                    // overlay position. We inject the callId so the handler knows
                    // the trace context. All other actions get their
                    // before-snapshot captured here.
                    if opts.snapshots && handler_captures_before(&method) {
                        cmd.params
                            .insert("_recordCallId".to_string(), Value::from(call_id.clone()));
                    }

                    let page_id = session.last_context.lock().unwrap().clone();
                    rec.record_action(&call_id, &method, &cmd.params, "", &page_id);
                }
            }

            // The after-snapshot only reads params["context"]; clone it before the
            // handler consumes cmd. (Go reads cmd.Params, whose map is shared.)
            let after_params = cmd.params.clone();
            let ctx_param = cmd
                .params
                .get("context")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            handler(Arc::clone(&router), Arc::clone(&session), cmd).await;

            // Capture endTime immediately after the handler returns, before
            // screenshot captures.
            let end_time = now_unix_millis();

            // Read and clear the element box stashed by
            // resolve_with_actionability / resolve_element.
            let box_ = session.last_element_box.lock().unwrap().take();

            if let Some(rec) = &recorder {
                if rec.is_recording() {
                    let opts = rec.options();

                    // Capture an after-snapshot to show the result of the action.
                    let mut after_snapshot = String::new();
                    if opts.snapshots {
                        after_snapshot = router
                            .capture_action_snapshot(&session, rec, &after_params, &call_id, "after")
                            .await;
                    }

                    // Skip if a handler already captured a screenshot (e.g. navigate).
                    let handler_captured_ss = session
                        .handler_screenshot
                        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok();
                    if opts.screenshots
                        && !handler_captured_ss
                        && session
                            .screenshot_in_flight
                            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                            .is_ok()
                    {
                        let ps = new_api_session(Arc::clone(&router), Arc::clone(&session), &ctx_param);
                        capture_recording_screenshot(&ps, rec, end_time).await;
                        session.screenshot_in_flight.store(false, Ordering::SeqCst);
                    }

                    rec.record_action_end(&call_id, &after_snapshot, end_time, box_);
                }
            }
        });
    }

    /// Spawns a handler concurrently WITHOUT acquiring `dispatch_mu`, mirroring
    /// Go's bare `go r.handleX(session, cmd)` for the network responder commands
    /// (continue/fulfill/abort) and the recording commands (start/stop/chunk/
    /// group). These must run while another command holds `dispatch_mu` — e.g. a
    /// `page.eval` blocked on `awaitPromise` for an intercepted `fetch()` can only
    /// resolve once the route handler's `fulfill` is processed; and the recording
    /// stop/chunk/group handlers acquire `dispatch_mu` themselves to order events,
    /// so going through `dispatch` (which holds it) would deadlock.
    ///
    /// Like `dispatch`, it registers the command as in-flight so the handler's
    /// `send_success`/`send_error` reply reaches the client (Go's bare goroutines
    /// all call `sendSuccess`/`sendError`; the JS client awaits these — e.g.
    /// `recording.stop` returns the trace zip).
    fn dispatch_concurrent<F>(self: &Arc<Self>, session: &Arc<BrowserSession>, cmd: BidiCommand, handler: F)
    where
        F: FnOnce(
                Arc<Router>,
                Arc<BrowserSession>,
                BidiCommand,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            + Send
            + 'static,
    {
        session.in_flight.lock().unwrap().insert(cmd.id);

        let router = Arc::clone(self);
        let session = Arc::clone(session);
        tokio::spawn(async move {
            handler(Arc::clone(&router), Arc::clone(&session), cmd).await;
        });
    }

    /// Called when a client disconnects. Closes the browser session.
    pub async fn on_client_disconnect(&self, client: &Arc<dyn ClientTransport>) {
        let session = self.sessions.lock().unwrap().remove(&client.id());
        if let Some(session) = session {
            self.close_session(&session).await;
        }
    }

    /// Reads messages from the browser and forwards them to the client.
    async fn route_browser_to_client(self: Arc<Self>, session: Arc<BrowserSession>) {
        loop {
            let msg = match session.bidi_conn.receive().await {
                Ok(m) => m,
                Err(e) => {
                    if !session.closed.load(Ordering::SeqCst) {
                        eprintln!(
                            "[router] Browser connection closed for client {}: {}",
                            session.client.id(),
                            e
                        );
                        self.sessions.lock().unwrap().remove(&session.client.id());
                        self.close_session(&session).await;
                        session.client.close();
                    }
                    return;
                }
            };

            // Route internal command responses; drop late timed-out ones.
            if let Ok(parsed) = serde_json::from_str::<Value>(&msg) {
                if let Some(id) = parsed.get("id").and_then(Value::as_i64) {
                    if id > 0 {
                        let tx = session.internal_cmds.lock().unwrap().remove(&id);
                        if let Some(tx) = tx {
                            let _ = tx.send(parsed);
                            continue;
                        }
                        if id >= 1_000_000 {
                            continue;
                        }
                    }
                }

                // Track page URL from load/navigation events.
                let method = parsed.get("method").and_then(Value::as_str).unwrap_or("");
                if method == "browsingContext.load" || method == "browsingContext.fragmentNavigated" {
                    if let Some(url) = parsed
                        .get("params")
                        .and_then(|p| p.get("url"))
                        .and_then(Value::as_str)
                    {
                        if !url.is_empty() {
                            *session.last_url.lock().unwrap() = url.to_string();
                        }
                    }
                }
            }

            // Record event for recording (non-blocking).
            let recorder = session.recorder.lock().unwrap().clone();
            if let Some(rec) = &recorder {
                if rec.is_recording() {
                    rec.record_bidi_event(&msg);
                }
            }

            // Check for WebSocket channel events (intercept, don't forward raw script.message).
            if self.is_ws_channel_event(&session, &msg) {
                continue;
            }

            // Forward message to client.
            if session.client.send(&msg).is_err() {
                eprintln!(
                    "[router] Failed to send to client {}",
                    session.client.id()
                );
                return;
            }
        }
    }

    /// Sends a BiDi command and waits for the response (60s timeout).
    pub async fn send_internal_command(
        &self,
        session: &Arc<BrowserSession>,
        method: &str,
        params: Value,
    ) -> anyhow::Result<Value> {
        self.send_internal_command_with_timeout(session, method, params, Duration::from_secs(60))
            .await
    }

    /// Sends a BiDi command and waits for the response with a custom timeout.
    pub async fn send_internal_command_with_timeout(
        &self,
        session: &Arc<BrowserSession>,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> anyhow::Result<Value> {
        // Record BiDi command in recording (opt-in via bidi: true). The matching
        // record_bidi_command_end runs after the response, like Go's `defer`.
        let rec_cmd = {
            let recorder = session.recorder.lock().unwrap().clone();
            match recorder {
                Some(rec) if rec.is_recording() && rec.options().bidi => {
                    let params_obj = params.as_object().cloned().unwrap_or_default();
                    let call_id = rec.record_bidi_command(method, &params_obj);
                    Some((rec, call_id))
                }
                _ => None,
            }
        };

        let id = session.next_internal_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        session.internal_cmds.lock().unwrap().insert(id, tx);

        let result = async {
            let cmd = json!({ "id": id, "method": method, "params": params });
            let cmd_bytes = serde_json::to_string(&cmd)?;
            if let Err(e) = session.bidi_conn.send(&cmd_bytes).await {
                session.internal_cmds.lock().unwrap().remove(&id);
                return Err(e);
            }

            match tokio::time::timeout(timeout, rx).await {
                Ok(Ok(resp)) => Ok(resp),
                Ok(Err(_)) => {
                    session.internal_cmds.lock().unwrap().remove(&id);
                    Err(anyhow!("session closed"))
                }
                Err(_) => {
                    session.internal_cmds.lock().unwrap().remove(&id);
                    Err(anyhow!("timeout waiting for response to {method}"))
                }
            }
        }
        .await;

        if let Some((rec, call_id)) = &rec_cmd {
            rec.record_bidi_command_end(call_id);
        }
        result
    }

    /// Retrieves the active browsing context (lastContext, else first from getTree).
    pub async fn get_context(&self, session: &Arc<BrowserSession>) -> anyhow::Result<String> {
        {
            let last = session.last_context.lock().unwrap().clone();
            if !last.is_empty() {
                return Ok(last);
            }
        }

        let resp = self
            .send_internal_command(session, "browsingContext.getTree", json!({}))
            .await?;

        let ctx = resp
            .get("result")
            .and_then(|r| r.get("contexts"))
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("context"))
            .and_then(Value::as_str);

        match ctx {
            Some(c) => Ok(c.to_string()),
            None => Err(anyhow!("no browsing contexts available")),
        }
    }

    /// Extracts the "context" param or returns the first context from getTree,
    /// storing the resolved context on the session.
    pub async fn resolve_context(
        &self,
        session: &Arc<BrowserSession>,
        params: &Map<String, Value>,
    ) -> anyhow::Result<String> {
        if let Some(ctx) = params.get("context").and_then(Value::as_str) {
            if !ctx.is_empty() {
                *session.last_context.lock().unwrap() = ctx.to_string();
                return Ok(ctx.to_string());
            }
        }
        let ctx = self.get_context(session).await?;
        *session.last_context.lock().unwrap() = ctx.clone();
        Ok(ctx)
    }

    /// Sends a successful response to the client. Sends exactly once per command
    /// id (first writer wins), so the close-time error flush never doubles up.
    pub(crate) fn send_success(&self, session: &Arc<BrowserSession>, id: i64, result: Value) {
        if !session.in_flight.lock().unwrap().remove(&id) {
            return;
        }
        let resp = BidiResponse {
            id,
            response_type: "success".to_string(),
            result: Some(result),
            error: String::new(),
            message: String::new(),
        };
        if let Ok(data) = serde_json::to_string(&resp) {
            let _ = session.client.send(&data);
        }
    }

    /// Sends an error response to the client (follows WebDriver BiDi spec).
    pub(crate) fn send_error(&self, session: &Arc<BrowserSession>, id: i64, err: &anyhow::Error) {
        if !session.in_flight.lock().unwrap().remove(&id) {
            return;
        }
        let message = err.to_string();
        let resp = BidiResponse {
            id,
            response_type: "error".to_string(),
            result: None,
            error: error_code(&message).to_string(),
            message,
        };
        if let Ok(data) = serde_json::to_string(&resp) {
            let _ = session.client.send(&data);
        }
    }

    /// Closes a browser session and cleans up resources.
    async fn close_session(&self, session: &Arc<BrowserSession>) {
        if session.closed.swap(true, Ordering::SeqCst) {
            return;
        }

        // Flush a terminal error for any in-flight command. In Go this surfaces
        // naturally because the handler's send hits the now-closed connection;
        // we reproduce the same observable reply deterministically.
        let pending: Vec<i64> = {
            let mut guard = session.in_flight.lock().unwrap();
            guard.drain().collect()
        };
        for id in pending {
            let resp = BidiResponse {
                id,
                response_type: "error".to_string(),
                result: None,
                error: error_code("connection closed").to_string(),
                message: "connection closed".to_string(),
            };
            if let Ok(data) = serde_json::to_string(&resp) {
                let _ = session.client.send(&data);
            }
        }

        eprintln!(
            "[router] Closing browser session for client {}",
            session.client.id()
        );

        // Stop screenshot loop before closing BiDi (captures use the connection).
        let recorder = session.recorder.lock().unwrap().clone();
        if let Some(rec) = &recorder {
            rec.stop_screenshots();
        }

        // Remote mode: end the BiDi session so chromedriver closes Chrome.
        if !self.connect_url.is_empty() {
            if let Some(client) = &session.bidi_client {
                let _ = client.send_command("session.end", json!({})).await;
            }
        }

        // Close the BiDi connection (this stops route_browser_to_client).
        let _ = session.bidi_conn.close().await;

        // Close the browser.
        if let Some(lr) = &session.launch_result {
            let _ = lr.close().await;
        }

        // Remove the download temp dir.
        let download_dir = session.download_dir.lock().unwrap().clone();
        if !download_dir.is_empty() {
            let _ = std::fs::remove_dir_all(&download_dir);
        }

        eprintln!(
            "[router] Browser session closed for client {}",
            session.client.id()
        );
    }

    /// Tears down `session` and then replies success to the `browserlane:browser.stop`
    /// command `cmd_id`. The reply is sent AFTER teardown so the client knows
    /// Chrome + chromedriver are fully terminated before it SIGTERMs the server
    /// (mirrors Go's `handleBrowserStop`: sessions.Delete → closeSession →
    /// sendSuccess).
    pub(crate) async fn stop_session_and_reply(&self, session: &Arc<BrowserSession>, cmd_id: i64) {
        // Stop routing new commands to this session.
        self.sessions.lock().unwrap().remove(&session.client.id());

        // Drop this command's in-flight token before closing: close_session
        // flushes a terminal "connection closed" error for every in-flight id
        // (a Rust-only mechanism Go lacks) and would otherwise hijack our reply.
        session.in_flight.lock().unwrap().remove(&cmd_id);
        self.close_session(session).await;

        // Re-arm the token and reply success now that cleanup is complete.
        session.in_flight.lock().unwrap().insert(cmd_id);
        self.send_success(session, cmd_id, json!({}));
    }

    /// Closes all browser sessions.
    pub async fn close_all(&self) {
        let sessions: Vec<Arc<BrowserSession>> = {
            let mut map = self.sessions.lock().unwrap();
            let v = map.values().cloned().collect();
            map.clear();
            v
        };
        for session in sessions {
            self.close_session(&session).await;
        }
    }
}

/// Returns true for interaction actions whose handlers capture the
/// before-snapshot after scrolling the element into view (via
/// resolve_with_actionability). For these, dispatch() injects `_recordCallId`
/// and the handler calls capture_before_snapshot_after_scroll between resolve and
/// act. Includes both click-like (before-only) and fill-like (before in handler
/// + after in dispatch) actions.
fn handler_captures_before(method: &str) -> bool {
    matches!(
        method,
        "browserlane:element.click"
            | "browserlane:element.dblclick"
            | "browserlane:element.hover"
            | "browserlane:element.tap"
            | "browserlane:element.check"
            | "browserlane:element.uncheck"
            | "browserlane:element.dragTo"
            | "browserlane:element.fill"
            | "browserlane:element.type"
            | "browserlane:element.press"
            | "browserlane:element.clear"
            | "browserlane:element.selectOption"
    )
}

/// Categorizes an error for clients. Only genuine timeouts are tagged "timeout".
fn error_code(message: &str) -> &'static str {
    if message.to_lowercase().contains("timeout") {
        "timeout"
    } else {
        "error"
    }
}
