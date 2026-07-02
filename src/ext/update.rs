//! `bl update` — self-update from the latest GitHub release.
//!
//! Downloads the platform asset, verifies it against SHA256SUMS, stops the
//! daemon, and replaces the running executable ATOMICALLY: the new binary is
//! staged next to the current one and moved into place with a rename, so the
//! target path always gets a fresh file (never an in-place overwrite). On
//! macOS that fresh file matters: overwritten binaries can inherit a stale
//! Gatekeeper verdict and get SIGKILLed on launch.

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sha2::{Digest, Sha256};

const REPO: &str = "browserlane/browserlane";

/// Rust target triple for the running platform, matching release asset names.
fn release_target() -> anyhow::Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        (os, arch) => Err(anyhow!(
            "no prebuilt binary for {os}/{arch} — build from source (see README)"
        )),
    }
}

fn http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(format!("browserlane-cli/{}", crate::VERSION))
        .build()
        .map_err(|e| anyhow!("failed to build HTTP client: {e}"))
}

/// Fetches the latest release tag (e.g. "v0.1.4") from the GitHub API.
async fn latest_tag(client: &reqwest::Client) -> anyhow::Result<String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = client.get(&url).send().await.map_err(|e| {
        anyhow!("could not reach GitHub to check for releases: {e}")
    })?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "GitHub API returned HTTP {} for {url}",
            resp.status().as_u16()
        ));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| anyhow!("failed to read the GitHub release response: {e}"))?;
    let body: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow!("failed to parse the GitHub release response: {e}"))?;
    body.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("GitHub release response had no tag_name"))
}

async fn download(client: &reqwest::Client, url: &str, dest: &Path) -> anyhow::Result<()> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow!("download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("HTTP {} for {url}", resp.status().as_u16()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| anyhow!("download failed mid-stream: {e}"))?;
    std::fs::write(dest, &bytes).map_err(|e| anyhow!("failed to write {}: {e}", dest.display()))?;
    Ok(())
}

/// Verifies `asset` in `dir` against a SHA256SUMS file from the release.
/// Missing SHA256SUMS or an unlisted asset is a note (matching install.sh);
/// a hash MISMATCH is a hard error.
async fn verify_checksum(client: &reqwest::Client, base: &str, dir: &Path, asset: &str) -> anyhow::Result<()> {
    let sums_path = dir.join("SHA256SUMS");
    if download(client, &format!("{base}/SHA256SUMS"), &sums_path).await.is_err() {
        eprintln!("note: SHA256SUMS not found — skipping checksum");
        return Ok(());
    }
    let sums = std::fs::read_to_string(&sums_path)?;
    let want = sums.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        match (fields.next(), fields.next()) {
            (Some(hash), Some(name)) if name == asset => Some(hash.to_lowercase()),
            _ => None,
        }
    });
    let Some(want) = want else {
        eprintln!("note: {asset} not listed in SHA256SUMS — skipping checksum");
        return Ok(());
    };
    let got = hex::encode(Sha256::digest(std::fs::read(dir.join(asset))?));
    if got != want {
        return Err(anyhow!("checksum mismatch for {asset} (expected {want}, got {got})"));
    }
    eprintln!("checksum ok");
    Ok(())
}

/// Minimal hex encoding (avoids a dependency for one call site).
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Extracts the release archive into `dir` and returns the path to the `bl`
/// binary inside it.
fn extract(archive: &Path, dir: &Path) -> anyhow::Result<PathBuf> {
    if archive.extension().and_then(|e| e.to_str()) == Some("zip") {
        let file = std::fs::File::open(archive)?;
        let mut zip = zip::ZipArchive::new(file)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let Some(rel) = entry.enclosed_name() else { continue }; // zip-slip guard
            let out = dir.join(rel);
            if entry.is_dir() {
                std::fs::create_dir_all(&out)?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
        }
    } else {
        // .tar.gz — the system tar, same tool install.sh already requires.
        let status = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(dir)
            .status()
            .map_err(|e| anyhow!("failed to run tar: {e}"))?;
        if !status.success() {
            return Err(anyhow!("tar failed to extract {}", archive.display()));
        }
    }

    let bin_name = if cfg!(windows) { "bl.exe" } else { "bl" };
    find_file(dir, bin_name)
        .ok_or_else(|| anyhow!("could not find {bin_name} inside the archive"))
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}

/// Replaces `exe` with `new_bin` via stage-next-to-it + rename, never an
/// in-place overwrite. Returns a friendly error when the directory isn't
/// writable.
fn replace_binary(new_bin: &Path, exe: &Path) -> anyhow::Result<()> {
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow!("cannot resolve the install directory of {}", exe.display()))?;
    let staged = dir.join(format!(".bl-update-{}", std::process::id()));

    std::fs::copy(new_bin, &staged).map_err(|e| {
        anyhow!(
            "no write access to {} ({e}) — re-run with the permissions that installed bl",
            dir.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }

    #[cfg(windows)]
    {
        // A running exe can't be deleted but CAN be renamed; park the old one
        // and clean it up on the next update.
        let old = dir.join("bl.exe.old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe, &old)
            .map_err(|e| anyhow!("failed to move the current bl.exe aside: {e}"))?;
        if let Err(e) = std::fs::rename(&staged, exe) {
            let _ = std::fs::rename(&old, exe); // roll back
            return Err(anyhow!("failed to install the new bl.exe: {e}"));
        }
        let _ = std::fs::remove_file(&old); // fails while running; next update sweeps it
    }
    #[cfg(not(windows))]
    if let Err(e) = std::fs::rename(&staged, exe) {
        let _ = std::fs::remove_file(&staged);
        return Err(anyhow!("failed to move the new binary into place: {e}"));
    }

    Ok(())
}

/// Runs `bl update`. `check_only` reports and exits (exit 1 when an update is
/// available); `force` reinstalls the latest release even when current.
pub async fn run(check_only: bool, force: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe()
        .map_err(|e| anyhow!("cannot resolve the running executable: {e}"))?;

    // Sweep a leftover from a previous Windows update, if any.
    #[cfg(windows)]
    if let Some(dir) = exe.parent() {
        let _ = std::fs::remove_file(dir.join("bl.exe.old"));
    }

    let client = http_client()?;
    let tag = latest_tag(&client).await?;
    let latest = tag.trim_start_matches('v');

    if latest == crate::VERSION && !force {
        println!("bl v{} is already the latest release.", crate::VERSION);
        return Ok(());
    }
    if check_only {
        println!("update available: v{} -> {tag}\nrun: bl update", crate::VERSION);
        std::process::exit(1);
    }

    let target = release_target()?;
    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    let asset = format!("bl-{tag}-{target}.{ext}");
    let base = format!("https://github.com/{REPO}/releases/download/{tag}");

    let tmp = std::env::temp_dir().join(format!("bl-update-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    let result = async {
        eprintln!("downloading {asset}");
        let archive = tmp.join(&asset);
        download(&client, &format!("{base}/{asset}"), &archive)
            .await
            .map_err(|e| anyhow!("download failed — is {tag} released for {target}? ({e})"))?;
        verify_checksum(&client, &base, &tmp, &asset).await?;
        let new_bin = extract(&archive, &tmp)?;

        // Stop the daemon so it doesn't keep running the old version.
        if crate::daemon::shutdown().await.is_ok() {
            eprintln!("stopped the running daemon");
        }

        replace_binary(&new_bin, &exe)?;
        println!(
            "updated bl v{} -> {tag} ({})",
            crate::VERSION,
            exe.display()
        );
        Ok(())
    }
    .await;
    let _ = std::fs::remove_dir_all(&tmp);
    result
}
