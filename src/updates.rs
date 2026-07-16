//! The only networking in LitePad, and it never runs on its own: every request
//! here is the direct result of the user clicking "Check for updates".
//!
//! The flow deliberately reuses the real installer rather than swapping the .exe
//! ourselves. `installer/litepad.iss` pins a constant `AppId`, so running
//! LitePad-Setup.exe upgrades the existing install in place — Start-menu and
//! desktop shortcuts, and the Apps & features uninstall entry, all survive.
//! (Overwriting our own .exe is not an option anyway: Windows locks a running
//! binary.)

use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::Sender;

use crate::storage::app_dir;

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/yashpandey0031/litepad/releases/latest";

/// The installer published by .github/workflows/release.yml. The same release
/// also carries a portable `litepad.exe`, so we match on the exact name — "the
/// first asset ending in .exe" would happily hand us the wrong file.
const INSTALLER_ASSET: &str = "LitePad-Setup.exe";

/// Compiled in from Cargo.toml, and compared against the release tag. These two
/// must move together — see the version guard in the release workflow.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// TLS via Windows' own schannel, so LitePad doesn't ship a second crypto stack.
fn agent() -> Result<ureq::Agent, String> {
    let tls = native_tls::TlsConnector::new()
        .map_err(|e| format!("Could not initialise TLS: {e}"))?;
    Ok(ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(tls))
        .user_agent("litepad-updater")
        .build())
}

/// Sent from a worker thread back to the UI.
pub enum UpdateMsg {
    UpToDate,
    Available { version: String, url: String },
    Downloaded(PathBuf),
    Failed(String),
}

/// Split "1.2.3" (or "v1.2.3", or "1.2.3-beta") into comparable numbers.
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let core = v.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next().unwrap_or("0").parse().ok()?;
    let patch: u32 = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        // An unparseable version is not a reason to push an update at someone.
        _ => false,
    }
}

/// Ask GitHub for the latest release. Runs on a worker thread.
pub fn check(tx: Sender<UpdateMsg>) {
    let _ = tx.send(match fetch_latest() {
        Ok(Some((version, url))) => UpdateMsg::Available { version, url },
        Ok(None) => UpdateMsg::UpToDate,
        Err(e) => UpdateMsg::Failed(e),
    });
}

fn fetch_latest() -> Result<Option<(String, String)>, String> {
    let body: serde_json::Value = agent()?
        .get(LATEST_RELEASE_API)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Could not reach GitHub: {e}"))?
        .into_json()
        .map_err(|e| format!("Unexpected response from GitHub: {e}"))?;

    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "GitHub returned a release with no tag".to_string())?;
    let latest = tag.trim_start_matches('v');

    if !is_newer(latest, CURRENT_VERSION) {
        return Ok(None);
    }

    let url = pick_installer(&body).ok_or_else(|| format!("Release {tag} has no {INSTALLER_ASSET}"))?;
    Ok(Some((latest.to_string(), url)))
}

/// Pull the installer's download URL out of a release payload, by exact name.
fn pick_installer(release: &serde_json::Value) -> Option<String> {
    release
        .get("assets")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(INSTALLER_ASSET))
        .and_then(|a| a.get("browser_download_url"))
        .and_then(|u| u.as_str())
        .map(str::to_string)
}

/// Fetch the installer to disk. Runs on a worker thread.
pub fn download(url: String, tx: Sender<UpdateMsg>) {
    let _ = tx.send(match fetch_installer(&url) {
        Ok(path) => UpdateMsg::Downloaded(path),
        Err(e) => UpdateMsg::Failed(e),
    });
}

fn fetch_installer(url: &str) -> Result<PathBuf, String> {
    let resp = agent()?
        .get(url)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let dir = app_dir().join("update");
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let path = dir.join(INSTALLER_ASSET);

    let mut file =
        fs::File::create(&path).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    io::copy(&mut resp.into_reader(), &mut file)
        .map_err(|e| format!("Download interrupted: {e}"))?;

    Ok(path)
}

/// Hand off to the installer's own wizard. The caller must close LitePad right
/// after: Inno cannot replace our .exe while it is still running.
pub fn run_installer(path: &PathBuf) -> Result<(), String> {
    Command::new(path)
        .spawn()
        .map_err(|e| format!("Could not start the installer: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.1.10", "0.1.9")); // not string ordering
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0")); // never downgrade
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn tolerates_tag_shapes() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    /// The release ships both LitePad-Setup.exe and a portable litepad.exe.
    /// Picking "the first .exe" would grab the wrong one — and on this payload
    /// the portable binary is listed first, so the ordering is not a safety net.
    #[test]
    fn picks_the_installer_not_the_portable_exe() {
        let release = serde_json::json!({
            "tag_name": "v0.2.0",
            "assets": [
                { "name": "litepad.exe",
                  "browser_download_url": "https://example.test/litepad.exe" },
                { "name": "LitePad-Setup.exe",
                  "browser_download_url": "https://example.test/LitePad-Setup.exe" }
            ]
        });
        assert_eq!(
            pick_installer(&release).as_deref(),
            Some("https://example.test/LitePad-Setup.exe")
        );
    }

    /// Hits the real GitHub API, so it's opt-in: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn live_check_reaches_github() {
        match fetch_latest() {
            Ok(None) => {} // running version is current — the usual case in CI
            Ok(Some((version, url))) => {
                assert!(url.ends_with(INSTALLER_ASSET), "got {url}");
                assert!(is_newer(&version, CURRENT_VERSION));
            }
            Err(e) => panic!("live check failed: {e}"),
        }
    }

    /// Downloads the real installer from the current release and checks we wrote a
    /// genuine executable — the streaming-to-disk path has no other coverage.
    /// Network + ~4 MB, so: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn live_download_lands_a_real_installer() {
        let url = format!(
            "https://github.com/yashpandey0031/litepad/releases/latest/download/{INSTALLER_ASSET}"
        );
        let path = fetch_installer(&url).expect("download should succeed");

        let bytes = fs::read(&path).expect("installer should be readable");
        assert!(bytes.len() > 1_000_000, "suspiciously small: {}", bytes.len());
        // "MZ" — a Windows executable, not an error page saved as a file.
        assert_eq!(&bytes[..2], b"MZ", "downloaded file is not a Windows .exe");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reports_a_release_with_no_installer() {
        let release = serde_json::json!({
            "tag_name": "v0.2.0",
            "assets": [{ "name": "litepad.exe", "browser_download_url": "https://example.test/x" }]
        });
        assert_eq!(pick_installer(&release), None);
    }
}
