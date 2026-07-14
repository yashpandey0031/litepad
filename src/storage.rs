//! On-disk persistence. Everything lives in a local folder — no network, ever.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Extensions we treat as editable plain-text notes.
const TEXT_EXTS: &[&str] = &["txt", "md", "markdown", "log", "text", "conf", "csv"];

/// `%APPDATA%\RustPad\notes` (created on demand).
pub fn notes_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("RustPad").join("notes");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// `%APPDATA%\RustPad` — root for config etc.
pub fn app_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("RustPad");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub struct LoadedNote {
    pub path: PathBuf,
    pub content: String,
    pub modified: SystemTime,
}

/// Load every supported text file in the notes folder, newest first.
pub fn load_all() -> Vec<LoadedNote> {
    let dir = notes_dir();
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !TEXT_EXTS.contains(&ext.as_str()) {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                let modified = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or_else(|_| SystemTime::now());
                out.push(LoadedNote {
                    path,
                    content,
                    modified,
                });
            }
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

/// Turn a note title into a safe Windows filename stem.
pub fn sanitize(title: &str) -> String {
    let trimmed = title.trim();
    let mut s: String = trimmed
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    // Collapse whitespace, cap length at a sane 60 chars.
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s = s.chars().take(60).collect::<String>().trim().to_string();
    // Windows disallows a few reserved stems and trailing dots.
    let reserved = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "lpt1", "lpt2", "lpt3",
    ];
    if s.is_empty() || reserved.contains(&s.to_lowercase().as_str()) {
        s = "Untitled".to_string();
    }
    s.trim_end_matches('.').to_string()
}

/// A path `<base>.<ext>` that doesn't collide with anything (except `keep`).
pub fn unique_path(dir: &Path, base: &str, ext: &str, keep: Option<&Path>) -> PathBuf {
    let first = dir.join(format!("{base}.{ext}"));
    if !first.exists() || keep == Some(first.as_path()) {
        return first;
    }
    let mut n = 2;
    loop {
        let candidate = dir.join(format!("{base} {n}.{ext}"));
        if !candidate.exists() || keep == Some(candidate.as_path()) {
            return candidate;
        }
        n += 1;
    }
}

/// Human-friendly relative time ("Just now", "5m ago", "Yesterday", "12 Jun").
pub fn relative_time(t: SystemTime) -> String {
    let now = SystemTime::now();
    let secs = now
        .duration_since(t)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match secs {
        0..=45 => "Just now".to_string(),
        46..=3599 => format!("{}m ago", (secs / 60).max(1)),
        3600..=86_399 => format!("{}h ago", secs / 3600),
        86_400..=172_799 => "Yesterday".to_string(),
        _ => format!("{}d ago", secs / 86_400),
    }
}
