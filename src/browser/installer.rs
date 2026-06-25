use std::collections::HashMap;
use std::path::Path;

use anyhow::anyhow;
use serde::Deserialize;

use crate::paths;

const LAST_KNOWN_GOOD_URL: &str =
    "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json";

/// Chrome for Testing version information.
#[derive(Debug, Clone, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    #[serde(default)]
    pub downloads: HashMap<String, Vec<Download>>,
}

/// A download URL for a specific platform.
#[derive(Debug, Clone, Deserialize)]
pub struct Download {
    pub platform: String,
    pub url: String,
}

/// API response for last known good versions.
#[derive(Debug, Deserialize)]
struct LastKnownGoodResponse {
    #[serde(default)]
    channels: HashMap<String, VersionInfo>,
}

/// Paths to installed binaries.
pub struct InstallResult {
    pub chrome_path: String,
    pub chromedriver_path: String,
    pub version: String,
}

/// Downloads and installs Chrome for Testing and chromedriver. Returns paths to
/// the installed binaries. Skips download if already installed.
pub async fn install() -> anyhow::Result<InstallResult> {
    if std::env::var("BROWSERLANE_SKIP_BROWSER_DOWNLOAD").as_deref() == Ok("1") {
        return Err(anyhow!(
            "browser download skipped (BROWSERLANE_SKIP_BROWSER_DOWNLOAD=1)"
        ));
    }

    if is_installed() {
        let chrome_path = paths::get_chrome_executable()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let chromedriver_path = paths::get_chromedriver_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let version = extract_version_from_path(&chrome_path);
        // Progress/status goes to stderr so stdout carries only the result — the
        // human summary or, under `--json`, a clean machine-parseable object.
        eprintln!("Chrome for Testing v{version} already installed.");
        return Ok(InstallResult {
            chrome_path,
            chromedriver_path,
            version,
        });
    }

    let platform = paths::get_platform_string();

    let version_info = fetch_latest_stable_version()
        .await
        .map_err(|e| anyhow!("failed to fetch version info: {e}"))?;

    eprintln!("Installing Chrome for Testing v{}...", version_info.version);

    let cft_dir = paths::get_chrome_for_testing_dir().map_err(|e| anyhow!("failed to get cache dir: {e}"))?;
    let version_dir = cft_dir.join(&version_info.version);
    std::fs::create_dir_all(&version_dir).map_err(|e| anyhow!("failed to create version dir: {e}"))?;

    let chrome_url = find_download_url(version_info.downloads.get("chrome"), &platform)
        .ok_or_else(|| anyhow!("no Chrome download available for platform {platform}"))?;
    eprintln!("Downloading Chrome from {chrome_url}...");
    download_and_extract(&chrome_url, &version_dir)
        .await
        .map_err(|e| anyhow!("failed to install Chrome: {e}"))?;

    let chromedriver_url = find_download_url(version_info.downloads.get("chromedriver"), &platform)
        .ok_or_else(|| anyhow!("no chromedriver download available for platform {platform}"))?;
    eprintln!("Downloading chromedriver from {chromedriver_url}...");
    download_and_extract(&chromedriver_url, &version_dir)
        .await
        .map_err(|e| anyhow!("failed to install chromedriver: {e}"))?;

    let chrome_path = paths::get_chrome_executable()
        .map_err(|e| anyhow!("Chrome installed but not found: {e}"))?;
    let chromedriver_path = paths::get_chromedriver_path()
        .map_err(|e| anyhow!("chromedriver installed but not found: {e}"))?;

    // Make executable on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&chrome_path, std::fs::Permissions::from_mode(0o755));
        let _ = std::fs::set_permissions(&chromedriver_path, std::fs::Permissions::from_mode(0o755));
    }

    // Remove quarantine attribute on macOS to avoid Gatekeeper prompts.
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(&chrome_path)
            .status();
        let _ = std::process::Command::new("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(&chromedriver_path)
            .status();
    }

    Ok(InstallResult {
        chrome_path: chrome_path.to_string_lossy().into_owned(),
        chromedriver_path: chromedriver_path.to_string_lossy().into_owned(),
        version: version_info.version,
    })
}

/// Fetches the latest stable Chrome for Testing version.
async fn fetch_latest_stable_version() -> anyhow::Result<VersionInfo> {
    let resp = reqwest::get(LAST_KNOWN_GOOD_URL).await?;
    if resp.status() != reqwest::StatusCode::OK {
        return Err(anyhow!("HTTP {}", resp.status().as_u16()));
    }
    let body = resp.bytes().await?;
    let data: LastKnownGoodResponse = serde_json::from_slice(&body)?;
    data.channels
        .get("Stable")
        .cloned()
        .ok_or_else(|| anyhow!("no Stable channel found"))
}

/// Finds the download URL for the given platform.
fn find_download_url(downloads: Option<&Vec<Download>>, platform: &str) -> Option<String> {
    downloads?
        .iter()
        .find(|d| d.platform == platform)
        .map(|d| d.url.clone())
}

/// Downloads a zip file and extracts it to the destination.
async fn download_and_extract(url: &str, dest_dir: &Path) -> anyhow::Result<()> {
    let resp = reqwest::get(url).await?;
    if resp.status() != reqwest::StatusCode::OK {
        return Err(anyhow!("HTTP {}", resp.status().as_u16()));
    }
    let bytes = resp.bytes().await?;

    // Write to a uniquely-named temp file (mirrors Go's os.CreateTemp).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = std::env::temp_dir().join(format!("chrome-{}-{}.zip", std::process::id(), nanos));
    std::fs::write(&tmp_path, &bytes)?;

    let result = extract_zip(&tmp_path, dest_dir);
    let _ = std::fs::remove_file(&tmp_path);
    result
}

/// Extracts a zip file to the destination directory.
fn extract_zip(zip_path: &Path, dest_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let raw_name = entry.name().to_string();

        // Strip the top-level directory (e.g. "chrome-mac-arm64/..." -> "...").
        let name = match raw_name.find('/') {
            Some(idx) => &raw_name[idx + 1..],
            None => raw_name.as_str(),
        };
        if name.is_empty() {
            continue;
        }

        let fpath = dest_dir.join(name);

        // Security check: prevent zip slip.
        if !fpath.starts_with(dest_dir) {
            return Err(anyhow!("invalid file path: {}", fpath.display()));
        }

        if entry.is_dir() {
            let _ = std::fs::create_dir_all(&fpath);
            continue;
        }

        if let Some(parent) = fpath.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = std::fs::File::create(&fpath)?;
        std::io::copy(&mut entry, &mut out)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                let _ = std::fs::set_permissions(&fpath, std::fs::Permissions::from_mode(mode));
            }
        }
    }

    Ok(())
}

/// Checks if Chrome for Testing and chromedriver are both installed.
/// Granular install state: Chrome's version (when its binary is present) and
/// whether chromedriver is present, reported independently so callers can say
/// *which* is missing. [`InstallStatus::installed`] collapses it back to the
/// all-or-nothing answer the install gate relies on.
pub struct InstallStatus {
    /// `Some(version)` when the Chrome for Testing binary exists, `None` when it
    /// is missing. The version is parsed from the cache path.
    pub chrome_version: Option<String>,
    /// Whether the chromedriver binary exists.
    pub chromedriver: bool,
}

impl InstallStatus {
    /// True only when BOTH binaries are present — the contract `bl is-installed`
    /// and the `install()` skip-check rely on.
    pub fn installed(&self) -> bool {
        self.chrome_version.is_some() && self.chromedriver
    }
}

/// Inspects the cache for Chrome for Testing + chromedriver, reporting each
/// independently (plus Chrome's version) so the CLI can give a precise answer.
pub fn install_status() -> InstallStatus {
    let chrome_version = paths::get_chrome_executable()
        .ok()
        .filter(|p| p.exists())
        .map(|p| extract_version_from_path(&p.to_string_lossy()));
    let chromedriver = paths::get_chromedriver_path()
        .map(|p| p.exists())
        .unwrap_or(false);
    InstallStatus {
        chrome_version,
        chromedriver,
    }
}

pub fn is_installed() -> bool {
    install_status().installed()
}

/// Extracts the version number from a Chrome path, e.g.
/// ".../chrome-for-testing/143.0.7499.192/..." -> "143.0.7499.192".
fn extract_version_from_path(path: &str) -> String {
    let parts: Vec<&str> = path.split(std::path::MAIN_SEPARATOR).collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "chrome-for-testing" && i + 1 < parts.len() {
            return parts[i + 1].to_string();
        }
    }
    "unknown".to_string()
}
