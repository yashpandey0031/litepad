//! Loads a handful of real Windows fonts at runtime (regular + bold variants).
//! Nothing is bundled into the binary — we read the system's own font files.

use std::fs;
use std::path::PathBuf;

use egui::{FontData, FontDefinitions, FontFamily};

/// (display name, group, regular file, bold file)
const CANDIDATES: &[(&str, &str, &str, &str)] = &[
    ("Segoe UI", "Modern", "segoeui.ttf", "segoeuib.ttf"),
    ("Arial", "Modern", "arial.ttf", "arialbd.ttf"),
    ("Georgia", "Classic", "georgia.ttf", "georgiab.ttf"),
    ("Consolas", "Classic", "consola.ttf", "consolab.ttf"),
];

pub struct FontChoice {
    pub label: String,
    pub group: &'static str,
    pub regular: FontFamily,
    pub bold: FontFamily,
}

fn fonts_dir() -> PathBuf {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    PathBuf::from(windir).join("Fonts")
}

/// Fall back to egui's bundled proportional + emoji fonts for missing glyphs.
fn fallback(key: &str) -> Vec<String> {
    vec![
        key.to_string(),
        "Ubuntu-Light".to_string(),
        "NotoEmoji-Regular".to_string(),
    ]
}

/// Register available fonts and return the pickable choices.
pub fn install(ctx: &egui::Context) -> Vec<FontChoice> {
    let mut defs = FontDefinitions::default();
    let dir = fonts_dir();
    let mut choices = Vec::new();

    for (label, group, reg_file, bold_file) in CANDIDATES {
        let Ok(reg_bytes) = fs::read(dir.join(reg_file)) else {
            continue;
        };
        let reg_key = format!("{label}-reg");
        defs.font_data
            .insert(reg_key.clone(), FontData::from_owned(reg_bytes));
        defs.families
            .insert(FontFamily::Name(reg_key.clone().into()), fallback(&reg_key));

        let bold = if let Ok(bold_bytes) = fs::read(dir.join(bold_file)) {
            let bold_key = format!("{label}-bold");
            defs.font_data
                .insert(bold_key.clone(), FontData::from_owned(bold_bytes));
            defs.families
                .insert(FontFamily::Name(bold_key.clone().into()), fallback(&bold_key));
            FontFamily::Name(bold_key.into())
        } else {
            FontFamily::Name(reg_key.clone().into())
        };

        choices.push(FontChoice {
            label: label.to_string(),
            group,
            regular: FontFamily::Name(reg_key.into()),
            bold,
        });
    }

    // Non-Windows / fonts missing: fall back to egui's built-ins so the app still runs.
    if choices.is_empty() {
        choices.push(FontChoice {
            label: "Sans".to_string(),
            group: "Modern",
            regular: FontFamily::Proportional,
            bold: FontFamily::Proportional,
        });
        choices.push(FontChoice {
            label: "Mono".to_string(),
            group: "Classic",
            regular: FontFamily::Monospace,
            bold: FontFamily::Monospace,
        });
    }

    ctx.set_fonts(defs);
    choices
}
