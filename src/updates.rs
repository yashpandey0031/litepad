//! Update checking and installation for LitePad.

use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::storage::app_dir;

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub checking: bool,
    pub last_check: Option<SystemTime>,
}

impl Default for UpdateInfo {
    fn default() -> Self {
        Self {
            available: false,
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            latest_version: env!("CARGO_PKG_VERSION").to_string(),
            download_url: String::new(),
            checking: false,
            last_check: None,
        }
    }
}

impl UpdateInfo {
    pub fn load() -> Self {
        let mut info = Self::default();
        let path = app_dir().join("updates.txt");
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let Some((k, v)) = line.split_once('=') else {
                    continue;
                };
                let (k, v) = (k.trim(), v.trim());
                match k {
                    "latest_version" => info.latest_version = v.to_string(),
                    "download_url" => info.download_url = v.to_string(),
                    "available" => info.available = v == "true",
                    _ => {}
                }
            }
        }
        info
    }

    pub fn save(&self) {
        let path = app_dir().join("updates.txt");
        let text = format!(
            "latest_version={}\ndownload_url={}\navailable={}\n",
            self.latest_version, self.download_url, self.available
        );
        let _ = fs::write(path, text);
    }
}

/// Check GitHub releases for a new version.
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let mut info = UpdateInfo::load();
    info.checking = true;
    info.last_check = Some(SystemTime::now());

    let client = Client::new();
    let url = "https://api.github.com/repos/yashpandey0031/litepad/releases/latest";

    match client
        .get(url)
        .header("User-Agent", "litepad-updater")
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(text) = response.text().await {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(tag) = json.get("tag_name").and_then(|v| v.as_str()) {
                        let latest = tag.trim_start_matches('v');
                        info.latest_version = latest.to_string();

                        // Find the Windows exe download
                        if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
                            for asset in assets {
                                if let Some(name) = asset.get("name").and_then(|v| v.as_str()) {
                                    if name.ends_with(".exe") {
                                        if let Some(url) =
                                            asset.get("browser_download_url").and_then(|v| v.as_str())
                                        {
                                            info.download_url = url.to_string();
                                            // Check if this is a new version
                                            if is_newer(latest, &info.current_version) {
                                                info.available = true;
                                            }
                                            info.save();
                                            return Ok(info);
                                        }
                                    }
                                }
                            }
                        }

                        info.save();
                        return Ok(info);
                    }
                }
            }
            Err("Failed to parse GitHub response".to_string())
        }
        Err(e) => Err(format!("Update check failed: {}", e)),
    }
}

/// Compare semantic versions: return true if `latest` > `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    use semver::Version;

    let parse_version = |v: &str| Version::parse(v).ok();

    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Download the update and save it to a temporary file. Returns the path to the downloaded file.
pub async fn download_update(url: &str) -> Result<PathBuf, String> {
    let client = Client::new();

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    let temp_dir = app_dir().join("temp");
    let _ = fs::create_dir_all(&temp_dir);

    let file_path = temp_dir.join("litepad_update.exe");
    fs::write(&file_path, bytes).map_err(|e| format!("Failed to save update: {}", e))?;

    Ok(file_path)
}

/// Replace the current executable with the new one and restart.
pub fn install_update(new_exe: &PathBuf) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current exe path: {}", e))?;

    let backup_exe = current_exe.with_extension("exe.bak");

    // Backup current exe
    fs::copy(&current_exe, &backup_exe)
        .map_err(|e| format!("Failed to backup current exe: {}", e))?;

    // Replace with new exe
    fs::copy(new_exe, &current_exe)
        .map_err(|e| format!("Failed to install update: {}", e))?;

    // Clean up temp file
    let _ = fs::remove_file(new_exe);

    // Restart the application
    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("Failed to restart app: {}", e))?;

    std::process::exit(0);
}
