//! Monochrome black/white theme. Solid colors only — no gradients anywhere.
//! Note: this styles the *UI chrome* only. The editor text font/size is handled
//! separately (see app.rs) so changing the editor font never touches the sidebar.

use egui::{Color32, FontFamily, FontId, Rounding, Stroke, TextStyle};

/// The available color themes. Cycled by the toolbar button.
#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    Brown,
}

impl Theme {
    /// Whether this theme is built on egui's dark base (affects widget defaults).
    pub fn is_dark(self) -> bool {
        !matches!(self, Theme::Light)
    }

    /// The next theme when the toggle is clicked: Light -> Dark -> Brown -> Light.
    pub fn next(self) -> Theme {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Brown,
            Theme::Brown => Theme::Light,
        }
    }

    /// Display name (also what the toggle button shows for the *next* theme).
    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::Brown => "Brown",
        }
    }

    pub fn from_str(s: &str) -> Theme {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Theme::Dark,
            "brown" => Theme::Brown,
            _ => Theme::Light,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub editor_bg: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub accent: Color32,      // selected note background
    pub accent_text: Color32, // text on the accent
    pub text: Color32,
    pub subtle: Color32,
    pub border: Color32,
    pub link: Color32,
    pub btn: Color32,
    pub btn_hover: Color32,
    pub btn_active: Color32,
}

impl Palette {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Palette {
                editor_bg: Color32::from_rgb(0x0E, 0x0E, 0x10),
                panel_bg: Color32::from_rgb(0x16, 0x16, 0x18),
                card_bg: Color32::from_rgb(0x1D, 0x1D, 0x20),
                accent: Color32::from_rgb(0xF2, 0xF2, 0xF4),
                accent_text: Color32::from_rgb(0x12, 0x12, 0x14),
                text: Color32::from_rgb(0xEC, 0xEC, 0xEE),
                subtle: Color32::from_rgb(0x8C, 0x8C, 0x93),
                border: Color32::from_rgb(0x2C, 0x2C, 0x31),
                link: Color32::from_rgb(0x6E, 0x9B, 0xFF),
                btn: Color32::from_rgb(0x24, 0x24, 0x28),
                btn_hover: Color32::from_rgb(0x30, 0x30, 0x36),
                btn_active: Color32::from_rgb(0x3B, 0x3B, 0x42),
            },
            Theme::Light => Palette {
                editor_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                panel_bg: Color32::from_rgb(0xF4, 0xF4, 0xF5),
                card_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                accent: Color32::from_rgb(0x11, 0x11, 0x13),
                accent_text: Color32::from_rgb(0xFA, 0xFA, 0xFA),
                text: Color32::from_rgb(0x18, 0x18, 0x1B),
                subtle: Color32::from_rgb(0x70, 0x70, 0x78),
                border: Color32::from_rgb(0xE2, 0xE2, 0xE5),
                link: Color32::from_rgb(0x1A, 0x66, 0xFF),
                btn: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                btn_hover: Color32::from_rgb(0xEC, 0xEC, 0xEE),
                btn_active: Color32::from_rgb(0xDD, 0xDD, 0xE0),
            },
            // Warm espresso: dark-based, brown tones, amber accents. Solid colors only.
            Theme::Brown => Palette {
                editor_bg: Color32::from_rgb(0x1F, 0x18, 0x11),
                panel_bg: Color32::from_rgb(0x27, 0x1E, 0x15),
                card_bg: Color32::from_rgb(0x32, 0x26, 0x19),
                accent: Color32::from_rgb(0xE7, 0xC9, 0xA0),
                accent_text: Color32::from_rgb(0x2A, 0x1D, 0x10),
                text: Color32::from_rgb(0xED, 0xE2, 0xD2),
                subtle: Color32::from_rgb(0xA9, 0x95, 0x7C),
                border: Color32::from_rgb(0x40, 0x30, 0x1F),
                link: Color32::from_rgb(0xF0, 0xB4, 0x58),
                btn: Color32::from_rgb(0x35, 0x28, 0x18),
                btn_hover: Color32::from_rgb(0x42, 0x33, 0x1F),
                btn_active: Color32::from_rgb(0x4F, 0x3D, 0x26),
            },
        }
    }
}

/// Push visuals + fixed UI text styling into the egui context.
pub fn apply(ctx: &egui::Context, pal: &Palette, theme: Theme) {
    let mut visuals = if theme.is_dark() {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    let round = Rounding::same(10.0);

    visuals.override_text_color = Some(pal.text);
    visuals.panel_fill = pal.panel_bg;
    visuals.window_fill = pal.panel_bg;
    visuals.extreme_bg_color = pal.editor_bg; // TextEdit background
    visuals.faint_bg_color = pal.card_bg;
    visuals.window_stroke = Stroke::new(1.0, pal.border);
    visuals.window_rounding = round;
    visuals.menu_rounding = round;
    // Text-selection highlight inside the editor (kept subtle, readable per theme).
    visuals.selection.bg_fill = match theme {
        Theme::Light => Color32::from_rgb(0xC7, 0xDB, 0xFF),
        Theme::Dark => Color32::from_rgb(0x33, 0x4A, 0x6B),
        Theme::Brown => Color32::from_rgb(0x5E, 0x46, 0x2A),
    };
    visuals.selection.stroke = Stroke::NONE;

    // Buttons: rounded, flat, gray tones — never the extreme black/white accent,
    // so clicking a button in dark mode no longer flashes a jarring color.
    for w in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
        &mut visuals.widgets.noninteractive,
    ] {
        w.rounding = round;
        w.fg_stroke = Stroke::new(1.0, pal.text);
    }
    visuals.widgets.inactive.bg_fill = pal.btn;
    visuals.widgets.inactive.weak_bg_fill = pal.btn;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.border);
    visuals.widgets.hovered.bg_fill = pal.btn_hover;
    visuals.widgets.hovered.weak_bg_fill = pal.btn_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.border);
    visuals.widgets.active.bg_fill = pal.btn_active;
    visuals.widgets.active.weak_bg_fill = pal.btn_active;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, pal.border);

    ctx.set_visuals(visuals);

    // Fixed UI text sizes — independent from the editor font/size.
    ctx.style_mut(|style| {
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(14.5, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::new(14.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::new(19.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace));
    });
}
