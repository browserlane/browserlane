/// Resolves once a shutdown signal (Ctrl-C / Ctrl-Break) is received. The unix
/// build also handles SIGTERM; on Windows tokio exposes Ctrl-C.
pub async fn wait_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
