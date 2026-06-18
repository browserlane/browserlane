//! Phase 3 clock cluster: the fake-clock routes (install/fastForward/runFor/
//! pauseAt/resume/setFixedTime/setSystemTime/setTimezone) plus the exported
//! timezone helpers used by the MCP agent. The injected JS lives in
//! `clock_script::CLOCK_SCRIPT`.

use std::sync::Arc;

use anyhow::anyhow;
use serde_json::{json, Value};

use super::clock_script::CLOCK_SCRIPT;
use super::helpers::{check_bidi_error, eval_simple_script};
use super::router::{BidiCommand, BrowserSession, Router};
use super::session::{new_api_session, Session};

impl Router {
    /// `browserlane:clock.install` — injects the fake clock and registers it as a
    /// preload script so it persists across navigations.
    pub(crate) async fn handle_clock_install(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        // Inject into the current page immediately.
        let s = new_api_session(Arc::clone(self), Arc::clone(session), &context);
        if let Err(e) = eval_simple_script(&s, &context, CLOCK_SCRIPT).await {
            self.send_error(session, cmd.id, &anyhow!("failed to install clock: {e}"));
            return;
        }

        // Register as a preload script (once per session) so it auto-runs on future navigations.
        let need_preload = session.clock_preload_script_id.lock().unwrap().is_empty();
        if need_preload {
            let resp = match self
                .send_internal_command(
                    session,
                    "script.addPreloadScript",
                    json!({
                        "functionDeclaration": CLOCK_SCRIPT,
                        "contexts": [context],
                    }),
                )
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    self.send_error(session, cmd.id, &anyhow!("failed to register clock preload: {e}"));
                    return;
                }
            };
            if let Err(e) = check_bidi_error(&resp) {
                self.send_error(session, cmd.id, &e);
                return;
            }
            let script_id = resp
                .get("result")
                .and_then(|r| r.get("script"))
                .and_then(Value::as_str)
                .unwrap_or("");
            *session.clock_preload_script_id.lock().unwrap() = script_id.to_string();
        }

        // If initial time is provided, set it.
        if let Some(time_val) = cmd.params.get("time").and_then(Value::as_f64) {
            let script =
                format!("() => {{ window.__browserlaneClock.setSystemTime({time_val}); return 'ok'; }}");
            if let Err(e) = eval_simple_script(&s, &context, &script).await {
                self.send_error(session, cmd.id, &anyhow!("failed to set initial time: {e}"));
                return;
            }
        }

        // If timezone is provided, override it via BiDi emulation.setTimezoneOverride.
        if let Some(tz) = cmd.params.get("timezone").and_then(Value::as_str) {
            if !tz.is_empty() {
                if let Err(e) = self.set_timezone_override(session, &context, tz).await {
                    self.send_error(session, cmd.id, &anyhow!("failed to set timezone: {e}"));
                    return;
                }
            }
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Evaluates a method call against the page's installed fake clock. Sends a
    /// clear error if the clock was never installed (`window.__browserlaneClock`
    /// undefined) instead of silently no-op'ing (issues #125, #137).
    async fn run_clock_op(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
        context: &str,
        op_name: &str,
        call: &str,
    ) {
        let s = new_api_session(Arc::clone(self), Arc::clone(session), context);
        let script = format!(
            "() => {{ if (!window.__browserlaneClock) return 'NOT_INSTALLED'; window.__browserlaneClock.{call}; return 'ok'; }}"
        );
        let res = match eval_simple_script(&s, context, &script).await {
            Ok(r) => r,
            Err(e) => {
                self.send_error(session, cmd.id, &anyhow!("clock.{op_name} failed: {e}"));
                return;
            }
        };
        if res == "NOT_INSTALLED" {
            self.send_error(
                session,
                cmd.id,
                &anyhow!("clock not installed: call clock.install() before clock.{op_name}()"),
            );
            return;
        }
        self.send_success(session, cmd.id, json!({}));
    }

    /// `browserlane:clock.fastForward` — jump forward N ms, fire due timers once.
    pub(crate) async fn handle_clock_fast_forward(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let ticks = match cmd.params.get("ticks").and_then(Value::as_f64) {
            Some(t) => t,
            None => {
                self.send_error(session, cmd.id, &anyhow!("ticks is required"));
                return;
            }
        };
        self.run_clock_op(session, cmd, &context, "fastForward", &format!("fastForward({ticks})"))
            .await;
    }

    /// `browserlane:clock.runFor` — advance N ms, fire all callbacks systematically.
    pub(crate) async fn handle_clock_run_for(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let ticks = match cmd.params.get("ticks").and_then(Value::as_f64) {
            Some(t) => t,
            None => {
                self.send_error(session, cmd.id, &anyhow!("ticks is required"));
                return;
            }
        };
        self.run_clock_op(session, cmd, &context, "runFor", &format!("runFor({ticks})"))
            .await;
    }

    /// `browserlane:clock.pauseAt` — jump to a time and pause.
    pub(crate) async fn handle_clock_pause_at(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let time = match cmd.params.get("time").and_then(Value::as_f64) {
            Some(t) => t,
            None => {
                self.send_error(session, cmd.id, &anyhow!("time is required"));
                return;
            }
        };
        self.run_clock_op(session, cmd, &context, "pauseAt", &format!("pauseAt({time})"))
            .await;
    }

    /// `browserlane:clock.resume` — resume real-time progression.
    pub(crate) async fn handle_clock_resume(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        self.run_clock_op(session, cmd, &context, "resume", "resume()").await;
    }

    /// `browserlane:clock.setFixedTime` — freeze Date.now() at a value.
    pub(crate) async fn handle_clock_set_fixed_time(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let time = match cmd.params.get("time").and_then(Value::as_f64) {
            Some(t) => t,
            None => {
                self.send_error(session, cmd.id, &anyhow!("time is required"));
                return;
            }
        };
        self.run_clock_op(session, cmd, &context, "setFixedTime", &format!("setFixedTime({time})"))
            .await;
    }

    /// `browserlane:clock.setSystemTime` — set Date.now() without firing timers.
    pub(crate) async fn handle_clock_set_system_time(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };
        let time = match cmd.params.get("time").and_then(Value::as_f64) {
            Some(t) => t,
            None => {
                self.send_error(session, cmd.id, &anyhow!("time is required"));
                return;
            }
        };
        self.run_clock_op(session, cmd, &context, "setSystemTime", &format!("setSystemTime({time})"))
            .await;
    }

    /// `browserlane:clock.setTimezone` — override or reset the browser timezone.
    pub(crate) async fn handle_clock_set_timezone(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let context = match self.resolve_context(session, &cmd.params).await {
            Ok(c) => c,
            Err(e) => {
                self.send_error(session, cmd.id, &e);
                return;
            }
        };

        let tz = cmd.params.get("timezone").and_then(Value::as_str).unwrap_or("");

        if tz.is_empty() {
            if let Err(e) = self.clear_timezone_override(session, &context).await {
                self.send_error(session, cmd.id, &anyhow!("failed to clear timezone: {e}"));
                return;
            }
        } else if let Err(e) = self.set_timezone_override(session, &context, tz).await {
            self.send_error(session, cmd.id, &anyhow!("failed to set timezone: {e}"));
            return;
        }

        self.send_success(session, cmd.id, json!({}));
    }

    /// Sets the browser timezone via BiDi `emulation.setTimezoneOverride`.
    async fn set_timezone_override(
        &self,
        session: &Arc<BrowserSession>,
        context: &str,
        timezone: &str,
    ) -> anyhow::Result<()> {
        let resp = self
            .send_internal_command(
                session,
                "emulation.setTimezoneOverride",
                json!({ "timezone": timezone, "contexts": [context] }),
            )
            .await?;
        check_bidi_error(&resp)
    }

    /// Resets the browser timezone to the system default.
    async fn clear_timezone_override(
        &self,
        session: &Arc<BrowserSession>,
        context: &str,
    ) -> anyhow::Result<()> {
        let resp = self
            .send_internal_command(
                session,
                "emulation.setTimezoneOverride",
                json!({ "timezone": null, "contexts": [context] }),
            )
            .await?;
        check_bidi_error(&resp)
    }
}

// ---------------------------------------------------------------------------
// Exported standalone clock/timezone functions — usable from both proxy and MCP.
// ---------------------------------------------------------------------------

/// Overrides the browser timezone via BiDi `emulation.setTimezoneOverride`.
pub async fn set_timezone(s: &dyn Session, context: &str, timezone: &str) -> anyhow::Result<()> {
    let resp = s
        .send_bidi_command(
            "emulation.setTimezoneOverride",
            json!({ "timezone": timezone, "contexts": [context] }),
        )
        .await?;
    check_bidi_error(&resp)
}

/// Resets the browser timezone to the system default.
pub async fn clear_timezone(s: &dyn Session, context: &str) -> anyhow::Result<()> {
    let resp = s
        .send_bidi_command(
            "emulation.setTimezoneOverride",
            json!({ "timezone": null, "contexts": [context] }),
        )
        .await?;
    check_bidi_error(&resp)
}
