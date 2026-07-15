//! Tiny key=value preferences file. No serde, no bloat.

use std::fs;

use crate::storage::app_dir;
use crate::theme::Theme;

#[derive(Clone)]
pub struct Config {
    pub theme: Theme,
    pub font_size: f32,
    pub font: String,
    pub sidebar_width: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            font_size: 17.0,
            font: "Segoe UI".to_string(),
            sidebar_width: 264.0,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Config::default();
        let path = app_dir().join("config.txt");
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let Some((k, v)) = line.split_once('=') else {
                    continue;
                };
                let (k, v) = (k.trim(), v.trim());
                match k {
                    "theme" => cfg.theme = Theme::from_str(v),
                    "font_size" => {
                        if let Ok(n) = v.parse::<f32>() {
                            cfg.font_size = n.clamp(11.0, 40.0);
                        }
                    }
                    "font" => {
                        if !v.is_empty() {
                            cfg.font = v.to_string();
                        }
                    }
                    "sidebar_width" => {
                        if let Ok(n) = v.parse::<f32>() {
                            cfg.sidebar_width = n.clamp(180.0, 480.0);
                        }
                    }
                    _ => {}
                }
            }
        }
        cfg
    }

    pub fn save(&self) {
        let path = app_dir().join("config.txt");
        let text = format!(
            "theme={}\nfont_size={}\nfont={}\nsidebar_width={}\n",
            self.theme.label().to_lowercase(),
            self.font_size,
            self.font,
            self.sidebar_width,
        );
        let _ = fs::write(path, text);
    }
}
