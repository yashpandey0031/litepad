use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::{Align, FontId, Layout, Margin, RichText, Rounding, Sense, Stroke, TextFormat};

use crate::config::Config;
use crate::fonts::FontChoice;
use crate::markup::{self, Fmt, Style};
use crate::storage;
use crate::theme::{self, Palette};
use crate::updates::UpdateInfo;

/// Save this long after the last keystroke.
const AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(700);

struct Note {
    id: u64,
    path: Option<PathBuf>,
    content: String,
    modified: SystemTime,
    dirty: bool,
    last_edit: Option<Instant>,
}

impl Note {
    /// First non-empty line, with formatting markers stripped.
    fn title(&self) -> String {
        self.content
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(|l| markup::strip_markers(l).chars().take(80).collect::<String>())
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "New note".to_string())
    }

    fn preview(&self) -> String {
        let mut lines = self.content.lines().map(str::trim).filter(|l| !l.is_empty());
        lines.next(); // skip the title line
        lines
            .next()
            .map(|l| markup::strip_markers(l).chars().take(90).collect::<String>())
            .unwrap_or_else(|| "No additional text".to_string())
    }

    fn ext(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "txt".to_string())
    }
}

/// Deferred UI actions, applied after the frame to keep borrows simple.
enum Action {
    New,
    Select(u64),
    RequestDelete(u64),
    Delete(u64),
    RevealFolder,
    SaveAs,
    CheckUpdates,
    InstallUpdate,
}

pub struct LitePadApp {
    notes: Vec<Note>,
    current: u64,
    next_id: u64,
    search: String,
    cfg: Config,
    palette: Palette,
    fonts: Vec<FontChoice>,
    theme_dirty: bool,
    focus_editor: bool,
    focus_search: bool,
    pending_format: Option<Fmt>,
    pending_delete: Option<u64>,
    show_shortcuts: bool,
    status: String,
    update_info: UpdateInfo,
    show_update_window: bool,
    downloading_update: bool,
}

impl LitePadApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let fonts = crate::fonts::install(&cc.egui_ctx);
        let cfg = Config::load();
        let palette = Palette::for_theme(cfg.theme);
        theme::apply(&cc.egui_ctx, &palette, cfg.theme);

        let mut next_id = 1u64;
        let mut notes: Vec<Note> = storage::load_all()
            .into_iter()
            .map(|n| {
                let id = next_id;
                next_id += 1;
                Note {
                    id,
                    path: Some(n.path),
                    content: n.content,
                    modified: n.modified,
                    dirty: false,
                    last_edit: None,
                }
            })
            .collect();

        if notes.is_empty() {
            notes.push(Note {
                id: next_id,
                path: None,
                content: String::new(),
                modified: SystemTime::now(),
                dirty: false,
                last_edit: None,
            });
            next_id += 1;
        }

        let current = notes[0].id;
        let update_info = UpdateInfo::load();

        Self {
            notes,
            current,
            next_id,
            search: String::new(),
            cfg,
            palette,
            fonts,
            theme_dirty: false,
            focus_editor: true,
            focus_search: false,
            pending_format: None,
            pending_delete: None,
            show_shortcuts: false,
            status: "Ready".to_string(),
            update_info,
            show_update_window: false,
            downloading_update: false,
        }
    }

    fn idx_of(&self, id: u64) -> Option<usize> {
        self.notes.iter().position(|n| n.id == id)
    }

    fn font_idx(&self) -> usize {
        self.fonts
            .iter()
            .position(|f| f.label == self.cfg.font)
            .unwrap_or(0)
    }

    fn new_note(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.notes.push(Note {
            id,
            path: None,
            content: String::new(),
            modified: SystemTime::now(),
            dirty: false,
            last_edit: None,
        });
        self.current = id;
        self.focus_editor = true;
        self.status = "New note".to_string();
    }

    fn mark_edited(&mut self, idx: usize) {
        self.notes[idx].dirty = true;
        self.notes[idx].last_edit = Some(Instant::now());
        self.notes[idx].modified = SystemTime::now();
        self.status = "Editing\u{2026}".to_string();
    }

    /// Write a note to disk, renaming the file if its title changed.
    fn save_note(&mut self, id: u64) {
        let Some(idx) = self.idx_of(id) else { return };

        if self.notes[idx].path.is_none() && self.notes[idx].content.trim().is_empty() {
            self.notes[idx].dirty = false;
            return;
        }

        let dir = storage::notes_dir();
        let ext = self.notes[idx].ext();
        let base = storage::sanitize(&self.notes[idx].title());
        let keep = self.notes[idx].path.clone();
        let desired = storage::unique_path(&dir, &base, &ext, keep.as_deref());

        if let Some(old) = keep.clone() {
            if old != desired {
                let _ = fs::rename(&old, &desired);
            }
        }

        match fs::write(&desired, &self.notes[idx].content) {
            Ok(_) => {
                self.notes[idx].path = Some(desired);
                self.notes[idx].modified = SystemTime::now();
                self.notes[idx].dirty = false;
                self.status = "All changes saved".to_string();
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    fn save_all_dirty(&mut self) {
        let ids: Vec<u64> = self.notes.iter().filter(|n| n.dirty).map(|n| n.id).collect();
        for id in ids {
            self.save_note(id);
        }
    }

    fn autosave_tick(&mut self) {
        let now = Instant::now();
        let due: Vec<u64> = self
            .notes
            .iter()
            .filter(|n| n.dirty)
            .filter(|n| {
                n.last_edit
                    .map(|t| now.duration_since(t) >= AUTOSAVE_DEBOUNCE)
                    .unwrap_or(true)
            })
            .map(|n| n.id)
            .collect();
        for id in due {
            self.save_note(id);
        }
    }

    fn delete_note(&mut self, id: u64) {
        if let Some(idx) = self.idx_of(id) {
            if let Some(path) = self.notes[idx].path.clone() {
                let _ = fs::remove_file(path);
            }
            self.notes.remove(idx);
            if self.notes.is_empty() {
                self.new_note();
            } else if self.current == id {
                let pick = idx.min(self.notes.len() - 1);
                self.current = self.notes[pick].id;
            }
            self.status = "Deleted".to_string();
        }
    }

    /// Export the current note to a user-chosen location via a native dialog.
    fn save_as(&mut self) {
        let Some(idx) = self.idx_of(self.current) else {
            return;
        };
        let base = storage::sanitize(&self.notes[idx].title());
        let ext = self.notes[idx].ext();
        let dialog = rfd::FileDialog::new()
            .set_file_name(format!("{base}.{ext}"))
            .add_filter("Text", &["txt", "md", "markdown", "log", "csv", "text"])
            .add_filter("All files", &["*"]);
        if let Some(path) = dialog.save_file() {
            match fs::write(&path, &self.notes[idx].content) {
                Ok(_) => self.status = format!("Saved a copy to {}", path.display()),
                Err(e) => self.status = format!("Save As failed: {e}"),
            }
        }
    }

    fn bump_font(&mut self, delta: f32) {
        self.cfg.font_size = (self.cfg.font_size + delta).clamp(11.0, 40.0);
        self.cfg.save();
    }

    fn request_format(&mut self, fmt: Fmt) {
        self.pending_format = Some(fmt);
        self.focus_editor = true;
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) -> Option<Action> {
        let mut action = None;
        let mut fmt = None;
        ctx.input_mut(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            if !ctrl {
                return;
            }
            if i.key_pressed(egui::Key::N) {
                action = Some(Action::New);
            }
            if i.key_pressed(egui::Key::S) {
                if i.modifiers.shift {
                    action = Some(Action::SaveAs);
                } else {
                    self.save_all_dirty();
                }
            }
            if i.key_pressed(egui::Key::F) {
                self.focus_search = true;
            }
            if i.key_pressed(egui::Key::D) {
                action = Some(Action::RequestDelete(self.current));
            }
            if i.key_pressed(egui::Key::B) {
                fmt = Some(Fmt::Bold);
            }
            if i.key_pressed(egui::Key::I) {
                fmt = Some(Fmt::Italic);
            }
            if i.key_pressed(egui::Key::U) {
                fmt = Some(Fmt::Underline);
            }
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                self.cfg.font_size = (self.cfg.font_size + 1.0).clamp(11.0, 40.0);
                self.cfg.save();
            }
            if i.key_pressed(egui::Key::Minus) {
                self.cfg.font_size = (self.cfg.font_size - 1.0).clamp(11.0, 40.0);
                self.cfg.save();
            }
        });
        if let Some(f) = fmt {
            self.request_format(f);
        }
        action
    }
}

impl eframe::App for LitePadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.theme_dirty {
            self.palette = Palette::for_theme(self.cfg.theme);
            theme::apply(ctx, &self.palette, self.cfg.theme);
            self.cfg.save();
            self.theme_dirty = false;
        }

        let mut action = self.handle_shortcuts(ctx);
        let pal = self.palette;
        let current_dirty = self
            .idx_of(self.current)
            .map(|i| self.notes[i].dirty)
            .unwrap_or(false);

        // ---- Top toolbar ----------------------------------------------------
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::none()
                    .fill(pal.panel_bg)
                    .inner_margin(Margin::symmetric(14.0, 9.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("LitePad").strong().size(16.0).color(pal.text));

                    ui.add_space(8.0);
                    // Fixed-width slot so the "Saving…" spinner and "Saved" state (which are
                    // different widths) never push the rest of the toolbar around.
                    ui.allocate_ui_with_layout(
                        egui::vec2(88.0, 18.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            if current_dirty {
                                ui.add(egui::Spinner::new().size(13.0));
                                ui.label(
                                    RichText::new("Saving\u{2026}").size(12.5).color(pal.subtle),
                                );
                            } else {
                                // Draw the check mark ourselves: the default UI font has no
                                // U+2713 glyph, so a literal "\u{2713}" renders as a tofu box.
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::vec2(15.0, 13.0), Sense::hover());
                                let c = rect.center();
                                ui.painter().add(egui::Shape::line(
                                    vec![
                                        c + egui::vec2(-4.5, 0.5),
                                        c + egui::vec2(-1.5, 3.3),
                                        c + egui::vec2(4.5, -3.4),
                                    ],
                                    Stroke::new(1.7, pal.subtle),
                                ));
                                ui.label(RichText::new("Saved").size(12.5).color(pal.subtle));
                            }
                        },
                    );

                    ui.add_space(2.0);
                    ui.separator();

                    // --- Editor font + inline formatting (applies to the SELECTION) ---
                    egui::ComboBox::from_id_salt("font_picker")
                        .selected_text(self.cfg.font.clone())
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for f in &self.fonts {
                                let label = format!("{}  \u{00b7}  {}", f.label, f.group);
                                if ui
                                    .selectable_label(f.label == self.cfg.font, label)
                                    .clicked()
                                {
                                    self.cfg.font = f.label.clone();
                                    self.cfg.save();
                                }
                            }
                        });

                    if ui
                        .button(RichText::new("B").strong())
                        .on_hover_text("Bold the selection (Ctrl+B)")
                        .clicked()
                    {
                        self.request_format(Fmt::Bold);
                    }
                    if ui
                        .button(RichText::new("I").italics())
                        .on_hover_text("Italicize the selection (Ctrl+I)")
                        .clicked()
                    {
                        self.request_format(Fmt::Italic);
                    }
                    if ui
                        .button(RichText::new("U").underline())
                        .on_hover_text("Underline the selection (Ctrl+U)")
                        .clicked()
                    {
                        self.request_format(Fmt::Underline);
                    }
                    if ui.button("A-").on_hover_text("Smaller text (Ctrl+-)").clicked() {
                        self.bump_font(-1.0);
                    }
                    if ui.button("A+").on_hover_text("Bigger text (Ctrl+=)").clicked() {
                        self.bump_font(1.0);
                    }

                    // --- Right-aligned actions ---
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let next_theme = self.cfg.theme.next();
                        if ui
                            .button(next_theme.label())
                            .on_hover_text("Switch theme (Light \u{2192} Dark \u{2192} Brown)")
                            .clicked()
                        {
                            self.cfg.theme = next_theme;
                            self.theme_dirty = true;
                        }
                        ui.separator();

                        // Update button - show with accent color if update available
                        let update_btn_text = if self.update_info.available {
                            RichText::new("Update").color(pal.accent_text)
                        } else if self.update_info.checking {
                            RichText::new("Checking\u{2026}")
                        } else {
                            RichText::new("Update")
                        };
                        let mut update_btn = egui::Button::new(update_btn_text);
                        if self.update_info.available {
                            update_btn = update_btn.fill(pal.accent);
                        }
                        if ui
                            .add(update_btn)
                            .on_hover_text(if self.update_info.available {
                                "New version available"
                            } else {
                                "Check for updates"
                            })
                            .clicked()
                        {
                            if self.update_info.available {
                                self.show_update_window = true;
                            } else {
                                action = Some(Action::CheckUpdates);
                            }
                        }

                        ui.separator();
                        // A real button (with a background) like Folder / Save As; filled
                        // with the accent while the shortcuts panel is open.
                        let mut shortcuts_btn = egui::Button::new(if self.show_shortcuts {
                            RichText::new("Shortcuts").color(pal.accent_text)
                        } else {
                            RichText::new("Shortcuts")
                        });
                        if self.show_shortcuts {
                            shortcuts_btn = shortcuts_btn.fill(pal.accent);
                        }
                        if ui
                            .add(shortcuts_btn)
                            .on_hover_text("Keyboard shortcuts")
                            .clicked()
                        {
                            self.show_shortcuts = !self.show_shortcuts;
                        }
                        if ui.button("Folder").on_hover_text("Open notes folder").clicked() {
                            action = Some(Action::RevealFolder);
                        }
                        if ui
                            .button("Save As\u{2026}")
                            .on_hover_text("Export a copy (Ctrl+Shift+S)")
                            .clicked()
                        {
                            action = Some(Action::SaveAs);
                        }
                    });
                });
            });

        // ---- Bottom status bar ---------------------------------------------
        let (words, chars) = self
            .idx_of(self.current)
            .map(|i| {
                let c = &self.notes[i].content;
                (c.split_whitespace().count(), c.chars().count())
            })
            .unwrap_or((0, 0));

        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::none()
                    .fill(pal.panel_bg)
                    .inner_margin(Margin::symmetric(14.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).size(12.0).color(pal.subtle));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{words} words \u{00b7} {chars} chars"))
                                .size(12.0)
                                .color(pal.subtle),
                        );
                    });
                });
            });

        // ---- Sidebar --------------------------------------------------------
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(self.cfg.sidebar_width)
            .width_range(190.0..=460.0)
            .frame(
                egui::Frame::none()
                    .fill(pal.panel_bg)
                    .inner_margin(Margin::symmetric(10.0, 10.0)),
            )
            .show(ctx, |ui| {
                self.cfg.sidebar_width = ui.available_width();

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Notes").strong().size(15.0).color(pal.text));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("+ New").size(13.0)))
                            .on_hover_text("New note (Ctrl+N)")
                            .clicked()
                        {
                            action = Some(Action::New);
                        }
                    });
                });

                ui.add_space(6.0);

                let search = egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search notes  (Ctrl+F)")
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(8.0, 6.0));
                let sr = ui.add(search);
                if self.focus_search {
                    sr.request_focus();
                    self.focus_search = false;
                }

                ui.add_space(8.0);

                let query = self.search.to_lowercase();
                let mut order: Vec<usize> = (0..self.notes.len()).collect();
                order.sort_by(|&a, &b| self.notes[b].modified.cmp(&self.notes[a].modified));

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                    )
                    .show(ui, |ui| {
                        let mut shown = 0;
                        for &i in &order {
                            let note = &self.notes[i];
                            if !query.is_empty() && !note.content.to_lowercase().contains(&query) {
                                continue;
                            }
                            shown += 1;
                            if let Some(a) = note_card(ui, note, note.id == self.current, &pal) {
                                action = Some(a);
                            }
                            ui.add_space(6.0);
                        }
                        if shown == 0 {
                            ui.add_space(20.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("No matching notes").color(pal.subtle).size(13.0),
                                );
                            });
                        }
                    });
            });

        // ---- Editor ---------------------------------------------------------
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(pal.editor_bg)
                    .inner_margin(Margin::symmetric(30.0, 22.0)),
            )
            .show(ctx, |ui| {
                let Some(idx) = self.idx_of(self.current) else {
                    return;
                };

                // Editor font is fully independent from the UI/sidebar styling.
                let fi = self.font_idx();
                let fid_reg = FontId::new(self.cfg.font_size, self.fonts[fi].regular.clone());
                let fid_bold = FontId::new(self.cfg.font_size, self.fonts[fi].bold.clone());
                let text_color = pal.text;
                let link_color = pal.link;

                let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                    let mut job = LayoutJob::default();
                    job.wrap.max_width = wrap_width;
                    build_job(&mut job, text, &fid_reg, &fid_bold, text_color, link_color);
                    ui.fonts(|f| f.layout_job(job))
                };

                let avail_h = ui.available_height();
                let rows = ((avail_h / (self.cfg.font_size * 1.5)).floor() as usize).max(8);
                let editor_id = egui::Id::new(("editor", self.current));

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let output = egui::TextEdit::multiline(&mut self.notes[idx].content)
                            .id(editor_id)
                            .frame(false)
                            .desired_width(f32::INFINITY)
                            .desired_rows(rows)
                            .layouter(&mut layouter)
                            .hint_text("Start writing\u{2026}")
                            .show(ui);

                        if output.response.changed() {
                            self.mark_edited(idx);
                        }
                        if self.focus_editor {
                            output.response.request_focus();
                            self.focus_editor = false;
                        }

                        // --- Apply a pending B/I/U to the current selection ---
                        if self.pending_format.is_some() {
                            if let Some(range) = output.cursor_range {
                                let fmt = self.pending_format.take().unwrap();
                                let a = range.primary.ccursor.index;
                                let b = range.secondary.ccursor.index;
                                let (start, end) = (a.min(b), a.max(b));

                                let (ns, ne) = markup::toggle_wrap(
                                    &mut self.notes[idx].content,
                                    start,
                                    end,
                                    fmt.marker(),
                                );
                                self.mark_edited(idx);

                                // Keep the same text selected after wrapping.
                                if let Some(mut state) =
                                    egui::text_edit::TextEditState::load(ctx, editor_id)
                                {
                                    state.cursor.set_char_range(Some(CCursorRange::two(
                                        CCursor::new(ns),
                                        CCursor::new(ne),
                                    )));
                                    state.store(ctx, editor_id);
                                }
                            }
                        }

                        // --- Ctrl+click to open a link ---
                        let ctrl = ctx.input(|i| i.modifiers.ctrl);
                        if ctrl {
                            if let Some(p) = output.response.hover_pos() {
                                let cur = output.galley.cursor_from_pos(p - output.galley_pos);
                                if markup::url_at(&self.notes[idx].content, cur.ccursor.index)
                                    .is_some()
                                {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            }
                            if output.response.clicked() {
                                if let Some(p) = output.response.interact_pointer_pos() {
                                    let cur = output.galley.cursor_from_pos(p - output.galley_pos);
                                    if let Some(url) =
                                        markup::url_at(&self.notes[idx].content, cur.ccursor.index)
                                    {
                                        ctx.open_url(egui::OpenUrl::same_tab(url));
                                    }
                                }
                            }
                        }
                    });
            });

        // ---- Shortcuts panel (next to Folder) ------------------------------
        if self.show_shortcuts {
            let mut open = true;
            egui::Window::new("Keyboard shortcuts")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::RIGHT_TOP, [-14.0, 58.0])
                .open(&mut open)
                .show(ctx, |ui| shortcuts_table(ui, &pal));
            self.show_shortcuts = open;
        }

        // ---- Delete confirmation modal -------------------------------------
        if let Some(id) = self.pending_delete {
            let title = self.idx_of(id).map(|i| self.notes[i].title()).unwrap_or_default();
            let mut open = true;
            egui::Window::new("Delete note")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(format!("Delete \u{201c}{title}\u{201d}?"));
                    ui.label(
                        RichText::new("This permanently removes the file.")
                            .size(12.0)
                            .color(pal.subtle),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.pending_delete = None;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Delete").color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(0xD0, 0x3A, 0x3A)),
                            )
                            .clicked()
                        {
                            action = Some(Action::Delete(id));
                            self.pending_delete = None;
                        }
                    });
                });
            if !open {
                self.pending_delete = None;
            }
        }

        // ---- Update available window ----------------------------------------
        if self.show_update_window {
            let mut open = true;
            egui::Window::new("Update Available")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Version {} is available",
                        self.update_info.latest_version
                    ));
                    ui.label(
                        RichText::new(format!(
                            "You are currently on version {}",
                            self.update_info.current_version
                        ))
                        .size(12.0)
                        .color(pal.subtle),
                    );
                    ui.add_space(10.0);

                    if self.downloading_update {
                        ui.add(egui::Spinner::new().size(16.0));
                        ui.label("Downloading update\u{2026}");
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_update_window = false;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Download & Install")
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(pal.accent),
                                )
                                .clicked()
                            {
                                action = Some(Action::InstallUpdate);
                                self.downloading_update = true;
                            }
                        });
                    }
                });
            if !open && !self.downloading_update {
                self.show_update_window = false;
            }
        }

        // ---- Apply deferred actions ----------------------------------------
        match action {
            Some(Action::New) => self.new_note(),
            Some(Action::Select(id)) => {
                self.save_all_dirty();
                self.current = id;
                self.focus_editor = true;
            }
            Some(Action::RequestDelete(id)) => self.pending_delete = Some(id),
            Some(Action::Delete(id)) => self.delete_note(id),
            Some(Action::SaveAs) => self.save_as(),
            Some(Action::RevealFolder) => {
                let dir = storage::notes_dir();
                let _ = std::process::Command::new("explorer").arg(dir).spawn();
            }
            Some(Action::CheckUpdates) => {
                self.status = "Checking for updates\u{2026}".to_string();
                self.update_info.checking = true;
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    match crate::updates::check_for_updates().await {
                        Ok(_info) => {
                            ctx.request_repaint();
                        }
                        Err(e) => {
                            eprintln!("Update check error: {}", e);
                        }
                    }
                });
            }
            Some(Action::InstallUpdate) => {
                let url = self.update_info.download_url.clone();
                tokio::spawn(async move {
                    match crate::updates::download_update(&url).await {
                        Ok(path) => {
                            if let Err(e) = crate::updates::install_update(&path) {
                                eprintln!("Update install error: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Download error: {}", e);
                        }
                    }
                });
            }
            None => {}
        }

        // ---- Autosave + graceful shutdown ----------------------------------
        self.autosave_tick();

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_all_dirty();
            self.cfg.save();
        }

        if self.notes.iter().any(|n| n.dirty) {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}

fn format_for(
    style: &Style,
    fid_reg: &FontId,
    fid_bold: &FontId,
    color: egui::Color32,
) -> TextFormat {
    TextFormat {
        font_id: if style.bold {
            fid_bold.clone()
        } else {
            fid_reg.clone()
        },
        color,
        italics: style.italic,
        underline: if style.underline {
            Stroke::new(1.0, color)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

/// Lay out the editor text: per-span bold/italic/underline, hidden markers, colored URLs.
fn build_job(
    job: &mut LayoutJob,
    text: &str,
    fid_reg: &FontId,
    fid_bold: &FontId,
    color: egui::Color32,
    link_color: egui::Color32,
) {
    for seg in markup::parse(text) {
        let s = &text[seg.start..seg.end];

        if seg.marker {
            // Hidden: the `**`/`*`/`__` characters stay in the layout so the galley's
            // char indices keep lining up with the underlying text buffer (cursor,
            // selection and editing all depend on that), but we render them fully
            // transparent at a near-zero size so they take no visible space.
            job.append(
                s,
                0.0,
                TextFormat {
                    font_id: FontId::new(0.1, fid_reg.family.clone()),
                    color: egui::Color32::TRANSPARENT,
                    ..Default::default()
                },
            );
            continue;
        }

        let base = format_for(&seg.style, fid_reg, fid_bold, color);
        let mut cur = 0usize;
        for (a, b) in markup::find_urls(s) {
            if a > cur {
                job.append(&s[cur..a], 0.0, base.clone());
            }
            let mut link = base.clone();
            link.color = link_color;
            link.underline = Stroke::new(1.0, link_color);
            job.append(&s[a..b], 0.0, link);
            cur = b;
        }
        if cur < s.len() {
            job.append(&s[cur..], 0.0, base.clone());
        }
    }
}

fn shortcuts_table(ui: &mut egui::Ui, pal: &Palette) {
    let rows = [
        ("New note", "Ctrl + N"),
        ("Save now", "Ctrl + S"),
        ("Save As\u{2026} (export)", "Ctrl + Shift + S"),
        ("Search notes", "Ctrl + F"),
        ("Delete note", "Ctrl + D"),
        ("Bold selection", "Ctrl + B"),
        ("Italic selection", "Ctrl + I"),
        ("Underline selection", "Ctrl + U"),
        ("Bigger / smaller text", "Ctrl + =  /  Ctrl + -"),
        ("Select all", "Ctrl + A"),
        ("Cut / Copy / Paste", "Ctrl + X / C / V"),
        ("Undo / Redo", "Ctrl + Z / Ctrl + Y"),
        ("Open link", "Ctrl + Click"),
    ];
    egui::Grid::new("shortcuts_grid")
        .num_columns(2)
        .spacing([24.0, 8.0])
        .show(ui, |ui| {
            for (action, keys) in rows {
                ui.label(RichText::new(action).color(pal.text).size(13.0));
                ui.label(RichText::new(keys).color(pal.subtle).monospace().size(12.5));
                ui.end_row();
            }
        });
}

/// Draw one note card. Returns `Select` on click; queues delete via context menu.
fn note_card(ui: &mut egui::Ui, note: &Note, selected: bool, pal: &Palette) -> Option<Action> {
    let bg = if selected { pal.accent } else { pal.card_bg };
    let title_color = if selected { pal.accent_text } else { pal.text };
    let sub_color = if selected {
        pal.accent_text.gamma_multiply(0.8)
    } else {
        pal.subtle
    };

    let inner = egui::Frame::none()
        .fill(bg)
        .rounding(Rounding::same(10.0))
        .stroke(if selected {
            Stroke::NONE
        } else {
            Stroke::new(1.0, pal.border)
        })
        .inner_margin(Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(note.title())
                        .strong()
                        .size(14.0)
                        .color(title_color),
                );
                ui.add_space(2.0);
                ui.label(RichText::new(note.preview()).size(12.0).color(sub_color));
                ui.add_space(3.0);
                ui.label(
                    RichText::new(storage::relative_time(note.modified))
                        .size(11.0)
                        .color(sub_color),
                );

                // File location for this note. Kept small and dim so it reads as metadata,
                // and truncated so a long path never widens the card. egui shows the full
                // path as a tooltip on hover automatically when the label is truncated.
                ui.add_space(2.0);
                let loc = note
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Not saved yet".to_string());
                ui.add(egui::Label::new(RichText::new(&loc).size(10.5).color(sub_color)).truncate());
            });
        });

    let resp = inner
        .response
        .interact(Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let mut result = None;
    if resp.clicked() {
        result = Some(Action::Select(note.id));
    }
    resp.context_menu(|ui| {
        if ui.button("Delete note").clicked() {
            result = Some(Action::RequestDelete(note.id));
            ui.close_menu();
        }
    });
    result
}
