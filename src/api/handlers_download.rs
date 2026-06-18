//! Phase 3 download cluster: per-session download setup (temp dir +
//! browser.setDownloadBehavior) and the `browserlane:download.saveAs` route that
//! copies a downloaded file out of the temp dir. The onDownload events
//! (browsingContext.downloadWillBegin/downloadEnd) are already subscribed +
//! forwarded by the router's event loop.

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use serde_json::{json, Value};

use super::router::{BidiCommand, BrowserSession, Router};

impl Router {
    /// Creates a temp dir and tells the browser to save downloads there.
    pub(crate) async fn setup_downloads(self: Arc<Self>, session: Arc<BrowserSession>) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("browserlane-downloads-{}-{}", std::process::id(), nanos));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("[router] Failed to create download temp dir: {e}");
            return;
        }
        let dir_str = dir.to_string_lossy().to_string();
        *session.download_dir.lock().unwrap() = dir_str.clone();

        if let Err(e) = self
            .send_internal_command(
                &session,
                "browser.setDownloadBehavior",
                json!({
                    "downloadBehavior": {
                        "type": "allowed",
                        "destinationFolder": dir_str,
                    }
                }),
            )
            .await
        {
            eprintln!("[router] Failed to set download behavior: {e}");
        }
    }

    /// `browserlane:download.saveAs` — copies a downloaded file from the temp dir to a
    /// user-specified path (with a path-traversal guard against the temp dir).
    pub(crate) async fn handle_download_save_as(
        self: &Arc<Self>,
        session: &Arc<BrowserSession>,
        cmd: BidiCommand,
    ) {
        let source_path = cmd.params.get("sourcePath").and_then(Value::as_str).unwrap_or("");
        let dest_path = cmd.params.get("destPath").and_then(Value::as_str).unwrap_or("");

        if source_path.is_empty() || dest_path.is_empty() {
            self.send_error(
                session,
                cmd.id,
                &anyhow!("download.saveAs requires sourcePath and destPath"),
            );
            return;
        }

        // Validate that the source is within the download dir (prevent path traversal).
        let dl_dir = session.download_dir.lock().unwrap().clone();

        // Mirror Go's filepath.Abs, which lexically cleans `..`/`.` after making the
        // path absolute. std::path::absolute does NOT collapse `..`, so without the
        // clean a sourcePath like `<dlDir>/../evil` would slip past the prefix check.
        let abs_source = match std::path::absolute(source_path) {
            Ok(p) => lexical_clean(&p.to_string_lossy()),
            Err(e) => {
                self.send_error(session, cmd.id, &anyhow!("invalid source path: {e}"));
                return;
            }
        };
        let abs_dl_dir = match std::path::absolute(&dl_dir) {
            Ok(p) => lexical_clean(&p.to_string_lossy()),
            Err(_) => lexical_clean(&dl_dir),
        };

        // Go: strings.HasPrefix(absSource, absDlDir+sep) || absSource == absDlDir.
        let sep = std::path::MAIN_SEPARATOR;
        let within = abs_source == abs_dl_dir || abs_source.starts_with(&format!("{abs_dl_dir}{sep}"));
        if !within {
            self.send_error(session, cmd.id, &anyhow!("source path is not within download directory"));
            return;
        }

        // Ensure the destination directory exists.
        if let Some(dest_dir) = Path::new(dest_path).parent() {
            if let Err(e) = std::fs::create_dir_all(dest_dir) {
                self.send_error(session, cmd.id, &anyhow!("failed to create destination directory: {e}"));
                return;
            }
        }

        // Copy the file (open src, create dst, copy) to mirror Go's error paths.
        let mut src = match std::fs::File::open(&abs_source) {
            Ok(f) => f,
            Err(e) => {
                self.send_error(session, cmd.id, &anyhow!("failed to open downloaded file: {e}"));
                return;
            }
        };
        let mut dst = match std::fs::File::create(dest_path) {
            Ok(f) => f,
            Err(e) => {
                self.send_error(session, cmd.id, &anyhow!("failed to create destination file: {e}"));
                return;
            }
        };
        if let Err(e) = std::io::copy(&mut src, &mut dst) {
            self.send_error(session, cmd.id, &anyhow!("failed to copy file: {e}"));
            return;
        }

        self.send_success(session, cmd.id, json!({ "saved": true }));
    }
}

/// Lexically cleans a path the way Go's `filepath.Clean` does on Unix: collapse
/// repeated separators, drop `.` elements, and resolve `..` against the preceding
/// element (or the root). Purely lexical — no filesystem or symlink resolution.
///
/// TODO(P8-2): replace with the shared cross-platform `filepath_abs`/`Clean`
/// helper once that lands.
fn lexical_clean(path: &str) -> String {
    const SEP: u8 = b'/';
    let bytes = path.as_bytes();
    let n = bytes.len();
    if n == 0 {
        return ".".to_string();
    }

    let rooted = bytes[0] == SEP;
    let mut out: Vec<u8> = Vec::with_capacity(n + 1);
    let mut r = 0usize;
    let mut dotdot = 0usize;
    if rooted {
        out.push(SEP);
        r = 1;
        dotdot = 1;
    }

    while r < n {
        if bytes[r] == SEP {
            // Empty path element (collapse repeated separators).
            r += 1;
        } else if bytes[r] == b'.' && (r + 1 == n || bytes[r + 1] == SEP) {
            // `.` element.
            r += 1;
        } else if bytes[r] == b'.'
            && bytes[r + 1] == b'.'
            && (r + 2 == n || bytes[r + 2] == SEP)
        {
            // `..` element: back up to the previous separator.
            r += 2;
            if out.len() > dotdot {
                let mut w = out.len() - 1;
                while w > dotdot && out[w] != SEP {
                    w -= 1;
                }
                out.truncate(w);
            } else if !rooted {
                if !out.is_empty() {
                    out.push(SEP);
                }
                out.push(b'.');
                out.push(b'.');
                dotdot = out.len();
            }
        } else {
            // Real path element: add a separator if needed, then copy it.
            if (rooted && out.len() != 1) || (!rooted && !out.is_empty()) {
                out.push(SEP);
            }
            while r < n && bytes[r] != SEP {
                out.push(bytes[r]);
                r += 1;
            }
        }
    }

    if out.is_empty() {
        out.push(b'.');
    }
    String::from_utf8(out).unwrap_or_else(|_| path.to_string())
}
