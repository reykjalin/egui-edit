use std::sync::mpsc::{channel, Receiver, Sender};

use egui::text::CCursorRange;
use egui::util::cache::{ComputerMut, FrameCache};
use egui::{
    text::{CursorRange, LayoutJob},
    text_selection::text_cursor_state::{ccursor_next_word, ccursor_previous_word},
    vec2, Align2, Event, EventFilter, FontId, Key, Margin, NumExt, Sense, Shape, TextBuffer, Vec2,
};
use egui::{Color32, Rect, TextFormat};
use relative_path::PathExt;

#[derive(Default)]
struct CodeHighlighter {}
impl ComputerMut<&str, LayoutJob> for CodeHighlighter {
    fn compute(&mut self, s: &str) -> LayoutJob {
        let mut job = LayoutJob::default();

        let words = s.split(' ');
        let word_count = words.clone().count();

        for (i, word) in words.enumerate() {
            let color = if i % 2 == 0 {
                Color32::LIGHT_RED
            } else {
                Color32::LIGHT_BLUE
            };

            job.append(
                word,
                0.0,
                TextFormat {
                    font_id: FontId::monospace(14.0),
                    color,
                    ..Default::default()
                },
            );

            if i != word_count - 1 {
                job.append(
                    " ",
                    0.0,
                    TextFormat {
                        font_id: FontId::monospace(14.0),
                        ..Default::default()
                    },
                );
            }
        }

        job.wrap.max_width = f32::INFINITY;
        job

        // LayoutJob::simple(
        //     s.into(),
        //     FontId::monospace(14.0),
        //     Color32::LIGHT_GRAY,
        //     f32::INFINITY,
        // )
    }
}
type HighlightCache = FrameCache<LayoutJob, CodeHighlighter>;

struct FileMessage {
    file: relative_path::RelativePathBuf,
    text: String,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    file: relative_path::RelativePathBuf,

    #[serde(skip)]
    cwd: std::path::PathBuf,

    #[serde(skip)]
    text: String,

    #[serde(skip)]
    selection: CursorRange,

    #[serde(skip)]
    file_channel: (Sender<FileMessage>, Receiver<FileMessage>),
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            file: relative_path::RelativePath::new(".").to_relative_path_buf(),
            cwd: std::env::current_dir().expect("Could not get current directory"),
            text: "".to_owned(),
            selection: CursorRange::default(),
            file_channel: channel(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cwd: std::path::PathBuf,
        file: Option<String>,
    ) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // FIXME: Merge JetBrains Mono font variants into one .ttf file to support
        // bolds/italic/boldi-italic etc.
        // See https://github.com/emilk/egui/discussions/1862.

        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "JetBrains Mono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/JetBrainsMonoNL-Regular.ttf")),
        );

        // Set JetBrains Mono as highest priority for monospaced fonts.
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "JetBrains Mono".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        // FIXME: This may be a slightly wonky way to open the file? The branching seems excessive at least.
        let config = if let Some(file) = file {
            let file = if std::path::Path::new(&file).is_absolute() {
                std::path::Path::new(&file)
                    .relative_to(cwd.clone())
                    .unwrap()
            } else {
                relative_path::RelativePath::new(&file).to_relative_path_buf()
            };

            // FIXME: All the PathBuf clone shenanigans here should probably be fixed by using some other type?

            // FIXME: Using .as_str() here to satisfy the borrow checker seems odd.
            // I'm probably doing something wrong here.
            let metadata = std::fs::metadata(file.to_path(cwd.clone())).unwrap_or_else(|_| {
                std::fs::File::create(file.to_path(cwd.clone()))
                    .expect("Could not create file")
                    .metadata()
                    .expect("Could not get metadata for file")
            });

            if metadata.is_file() {
                // FIXME: Inform user of file read error more gracefully than with a panic.
                let text =
                    std::fs::read_to_string(file.to_path(cwd.clone())).unwrap_or_else(|_| {
                        panic!(
                            "Could not read file {}",
                            file.to_path(cwd.clone()).to_string_lossy()
                        )
                    });

                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::Title(format!(
                        "egui_edit - {}",
                        file.as_str()
                    )));

                Self {
                    text,
                    file: file.to_relative_path_buf(),
                    ..Default::default()
                }
            } else {
                // FXIME: we should create the file here.
                Default::default()
            }
        } else {
            Default::default()
        };

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            // FIXME: Is there a better way to merge these structs? This seems slightly off.
            return Self {
                text: config.text,
                file: config.file,
                ..eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            };
        }

        config
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        if let Ok(msg) = self.file_channel.1.try_recv() {
            self.text = msg.text;
            self.file = msg.file;

            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "egui_edit - {}",
                self.file.as_str()
            )));
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(egui::Button::new("Open…").shortcut_text(
                            egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, Key::O).format(
                                &egui::ModifierNames::SYMBOLS,
                                egui::os::OperatingSystem::from_target_os()
                                    == egui::os::OperatingSystem::Mac,
                            ),
                        ))
                        .clicked()
                    {
                        open_file_with_native_dialog(
                            ui,
                            self.file_channel.0.clone(),
                            self.cwd.clone(),
                        );
                        ui.close_menu();
                    }

                    if ui
                        .add(egui::Button::new("Save file").shortcut_text(
                            egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, Key::S).format(
                                &egui::ModifierNames::SYMBOLS,
                                egui::os::OperatingSystem::from_target_os()
                                    == egui::os::OperatingSystem::Mac,
                            ),
                        ))
                        .clicked()
                    {
                        save_text_to_file(self.file.as_str(), self.text.as_str());
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
                ui.add_space(16.0);
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // The bottom panel is often a good place for toolbars and status bars:

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            egui::ScrollArea::both().show(ui, |ui| {
                // =============================
                // Set up the available layout.
                // =============================
                let where_to_put_background = ui.painter().add(Shape::Noop);
                let margin = Margin::symmetric(4.0, 2.0);
                let available = ui.available_rect_before_wrap();
                let max_rect = margin.shrink_rect(available);
                let mut content_ui = ui.child_ui(max_rect, egui::Layout::default());

                let font_id = FontId::new(14.0, egui::FontFamily::Monospace);

                // =============================
                // Layout function for the text, incl. syntax highlighting.
                // =============================
                let layouter = |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                    let layout_job = ui
                        .ctx()
                        .memory_mut(|m| m.caches.cache::<HighlightCache>().get(text));

                    ui.fonts(|f| f.layout_job(layout_job))
                };

                // =============================
                // Calculate dimensions.
                // =============================
                let row_height = content_ui.fonts(|f| f.row_height(&font_id));

                const MIN_WIDTH: f32 = 24.0;
                let available_width = content_ui.available_width().at_least(MIN_WIDTH);
                let wrap_width = available_width;

                let mut galley = layouter(&content_ui, &self.text, wrap_width);

                // Clip all text.
                let desired_width = available_width;
                let desired_height = content_ui.available_height().at_least(row_height);
                // Default values form the TextGui TextEdit.
                let at_least = Vec2::ZERO - Margin::symmetric(4.0, 2.0).sum();
                let desired_size = vec2(
                    galley.size().x.max(desired_width),
                    galley.size().y.max(desired_height),
                )
                .at_least(at_least);

                let (id, rect) = content_ui.allocate_space(desired_size);

                let painter = content_ui.painter_at(rect.expand(1.0)); // expand to avoid clipping cursor.

                let galley_pos = Align2::LEFT_TOP
                    .align_size_within_rect(galley.size(), rect)
                    .intersect(rect) // limit pos to the response rect area
                    .min;

                // ---
                // Cursor positions and dimensions.
                // ---

                // FIXME: support multiple cursors/selections.
                let mut cursor_pos = galley
                    .pos_from_cursor(&self.selection.primary)
                    .translate(galley_pos.to_vec2());

                // Handle completely empty galleys
                cursor_pos.max.y = cursor_pos.max.y.at_least(cursor_pos.min.y + row_height);
                // Expand to slightly above and below the text.
                cursor_pos = cursor_pos.expand(1.5);

                let cursor_stroke = ui.visuals().text_cursor;
                let top = cursor_pos.center_top();
                let bottom = cursor_pos.center_bottom();

                // Turn on IME if we have focus.
                // IME is supposed to be on when the user is editing text.
                if content_ui.memory(|m| m.has_focus(id)) {
                    content_ui.output_mut(|o| {
                        o.ime = Some(egui::output::IMEOutput {
                            rect,
                            cursor_rect: cursor_pos,
                        })
                    })
                }

                // =============================
                // Do interactions.
                // =============================
                let sense = Sense::click_and_drag();
                let mut response = content_ui.interact(rect, id, sense);

                // ---
                // Mouse interactions.
                // ---
                if let Some(_pointer_pos) = content_ui.ctx().pointer_interact_pos() {
                    if response.hovered() {
                        content_ui.output_mut(|o| o.mutable_text_under_cursor = true);
                    }
                }

                if response.hovered() {
                    content_ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }

                if response.clicked() {
                    content_ui.memory_mut(|m| m.request_focus(response.id));
                }

                if let Some(pointer_pos) = content_ui.ctx().pointer_interact_pos() {
                    if response.is_pointer_button_down_on() && response.dragged() {
                        self.selection = CursorRange {
                            primary: galley.cursor_from_pos(pointer_pos - response.rect.min),
                            secondary: self.selection.secondary,
                        };
                    } else if response.is_pointer_button_down_on() {
                        self.selection = CursorRange {
                            primary: galley.cursor_from_pos(pointer_pos - response.rect.min),
                            secondary: galley.cursor_from_pos(pointer_pos - response.rect.min),
                        };
                    }
                }

                // ---
                // Keyboard interactions.
                // ---

                let event_filter = EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    tab: true,
                    ..Default::default()
                };

                if content_ui.memory(|m| m.has_focus(id)) {
                    let events = content_ui.input(|i| i.filtered_events(&event_filter));

                    content_ui.memory_mut(|m| m.set_focus_lock_filter(id, event_filter));

                    for event in &events {
                        let new_ccursor_range = match event {
                            Event::Text(text_to_insert) => {
                                if !text_to_insert.is_empty()
                                    && text_to_insert != "\n"
                                    && text_to_insert != "\r"
                                {
                                    let mut ccursor = self.text.delete_selected(&self.selection);
                                    self.text.insert_text_at(
                                        &mut ccursor,
                                        text_to_insert,
                                        usize::MAX,
                                    );

                                    Some(CCursorRange::one(ccursor))
                                } else {
                                    None
                                }
                            }
                            Event::Key {
                                key: Key::Tab,
                                pressed: true,
                                ..
                            } => {
                                let mut ccursor = self.text.delete_selected(&self.selection);
                                self.text.insert_text_at(&mut ccursor, "\t", usize::MAX);

                                Some(CCursorRange::one(ccursor))
                            }
                            Event::Key {
                                key: Key::Enter,
                                pressed: true,
                                ..
                            } => {
                                let mut ccursor = self.text.delete_selected(&self.selection);
                                self.text.insert_text_at(&mut ccursor, "\n", usize::MAX);

                                Some(CCursorRange::one(ccursor))
                            }
                            Event::Key {
                                key: Key::Backspace,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                let ccursor = if modifiers.mac_cmd {
                                    self.text
                                        .delete_paragraph_before_cursor(&galley, &self.selection)
                                } else if let Some(cursor) = self.selection.single() {
                                    if modifiers.alt {
                                        self.text.delete_previous_word(cursor.ccursor)
                                    } else {
                                        self.text.delete_previous_char(cursor.ccursor)
                                    }
                                } else {
                                    self.text.delete_selected(&self.selection)
                                };

                                Some(CCursorRange::one(ccursor))
                            }
                            Event::Key {
                                key: Key::ArrowLeft,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                if modifiers.is_none() {
                                    Some(CCursorRange::one(
                                        galley
                                            .cursor_left_one_character(&self.selection.primary)
                                            .ccursor,
                                    ))
                                } else if modifiers.alt {
                                    Some(CCursorRange::one(ccursor_previous_word(
                                        &self.text,
                                        self.selection.primary.ccursor,
                                    )))
                                } else if modifiers.mac_cmd {
                                    Some(CCursorRange::one(
                                        galley.cursor_begin_of_row(&self.selection.primary).ccursor,
                                    ))
                                } else if modifiers.shift {
                                    Some(CCursorRange::two(
                                        self.selection.secondary.ccursor,
                                        galley
                                            .cursor_left_one_character(&self.selection.primary)
                                            .ccursor,
                                    ))
                                } else {
                                    None
                                }
                            }
                            Event::Key {
                                key: Key::ArrowRight,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                if modifiers.is_none() {
                                    Some(CCursorRange::one(
                                        galley
                                            .cursor_right_one_character(&self.selection.primary)
                                            .ccursor,
                                    ))
                                } else if modifiers.alt {
                                    Some(CCursorRange::one(ccursor_next_word(
                                        &self.text,
                                        self.selection.primary.ccursor,
                                    )))
                                } else if modifiers.mac_cmd {
                                    Some(CCursorRange::one(
                                        galley.cursor_end_of_row(&self.selection.primary).ccursor,
                                    ))
                                } else if modifiers.shift {
                                    Some(CCursorRange::two(
                                        self.selection.secondary.ccursor,
                                        galley
                                            .cursor_right_one_character(&self.selection.primary)
                                            .ccursor,
                                    ))
                                } else {
                                    None
                                }
                            }
                            Event::Key {
                                key: Key::ArrowUp,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                if modifiers.is_none() {
                                    Some(CCursorRange::one(
                                        galley.cursor_up_one_row(&self.selection.primary).ccursor,
                                    ))
                                } else if modifiers.mac_cmd {
                                    Some(CCursorRange::one(galley.begin().ccursor))
                                } else if modifiers.shift {
                                    Some(CCursorRange::two(
                                        self.selection.secondary.ccursor,
                                        galley.cursor_up_one_row(&self.selection.primary).ccursor,
                                    ))
                                } else {
                                    None
                                }
                            }
                            Event::Key {
                                key: Key::ArrowDown,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                if modifiers.is_none() {
                                    Some(CCursorRange::one(
                                        galley.cursor_down_one_row(&self.selection.primary).ccursor,
                                    ))
                                } else if modifiers.mac_cmd {
                                    Some(CCursorRange::one(galley.end().ccursor))
                                } else if modifiers.shift {
                                    Some(CCursorRange::two(
                                        self.selection.secondary.ccursor,
                                        galley.cursor_down_one_row(&self.selection.primary).ccursor,
                                    ))
                                } else {
                                    None
                                }
                            }
                            Event::Key {
                                key: Key::O,
                                pressed: true,
                                modifiers,
                                ..
                            } if modifiers.command_only() => {
                                open_file_with_native_dialog(
                                    &content_ui,
                                    self.file_channel.0.clone(),
                                    self.cwd.clone(),
                                );
                                None
                            }
                            Event::Key {
                                key: Key::S,
                                pressed: true,
                                modifiers,
                                ..
                            } if modifiers.command_only() => {
                                save_text_to_file(self.file.as_str(), self.text.as_str());
                                None
                            }
                            _ => None,
                        };

                        if let Some(new_ccursor_range) = new_ccursor_range {
                            galley = layouter(&content_ui, &self.text, wrap_width);
                            self.selection = CursorRange {
                                primary: galley.from_ccursor(new_ccursor_range.primary),
                                secondary: galley.from_ccursor(new_ccursor_range.secondary),
                            };

                            // Scroll to the cursor to make sure it's in view after its position changed.
                            content_ui.scroll_to_rect(cursor_pos, None)
                        }
                    }
                }

                // =============================
                // Draw the text.
                // =============================
                if content_ui.is_rect_visible(rect) {
                    painter.galley(galley_pos, galley.clone(), egui::Color32::WHITE);
                }

                // =============================
                // Draw the cursor.
                // =============================

                // The selection.
                if !self.selection.is_empty() {
                    // We paint the cursor selection on top of the text, so make it transparent:
                    let color = content_ui.visuals().selection.bg_fill.linear_multiply(0.5);
                    let [min, max] = self.selection.sorted_cursors();
                    let min = min.rcursor;
                    let max = max.rcursor;

                    for ri in min.row..=max.row {
                        let row = &galley.rows[ri];
                        let left = if ri == min.row {
                            row.x_offset(min.column)
                        } else {
                            row.rect.left()
                        };
                        let right = if ri == max.row {
                            row.x_offset(max.column)
                        } else {
                            let newline_size = if row.ends_with_newline {
                                row.height() / 2.0 // visualize that we select the newline
                            } else {
                                0.0
                            };
                            row.rect.right() + newline_size
                        };
                        let rect = Rect::from_min_max(
                            galley_pos + vec2(left, row.min_y()),
                            galley_pos + vec2(right, row.max_y()),
                        );

                        painter.rect_filled(rect, 0.0, color);
                    }
                }

                // The cursor itself.
                if content_ui.memory(|m| m.has_focus(id)) {
                    painter.line_segment([top, bottom], (cursor_stroke.width, cursor_stroke.color));
                }

                // =============================
                // Draw border and background.
                // =============================
                let frame_id = response.id;
                let frame_rect = margin.expand_rect(response.rect);
                ui.allocate_space(frame_rect.size());
                response |= ui.interact(frame_rect, frame_id, Sense::click());
                if response.clicked() && !response.lost_focus() {
                    ui.memory_mut(|mem| mem.request_focus(response.id));
                }

                let visuals = ui.style().interact(&response);
                let frame_rect = frame_rect.expand(visuals.expansion);
                let active_stroke = ui.visuals().selection.stroke;
                let inactive_stroke = ui.visuals().selection.stroke; // Probably want a fainter version of the active stroke color.
                let shape = if response.has_focus() {
                    epaint::RectShape::new(
                        frame_rect,
                        0.0,
                        ui.visuals().extreme_bg_color,
                        active_stroke,
                    )
                } else {
                    epaint::RectShape::new(
                        frame_rect,
                        0.0,
                        ui.visuals().extreme_bg_color,
                        inactive_stroke,
                    )
                };

                ui.painter().set(where_to_put_background, shape);

                // FIXME: Fix accesskit integration.
                #[cfg(feature = "accesskit")]
                {
                    let role = accesskit::Role::MultilineTextInput;
                    crate::text_selection::accesskit_text::update_accesskit_for_text_widget(
                        ui.ctx(),
                        id,
                        cursor_range,
                        role,
                        galley_pos,
                        &galley,
                    );
                }
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}

fn open_file_with_native_dialog(
    ui: &egui::Ui,
    sender: Sender<FileMessage>,
    cwd: std::path::PathBuf,
) {
    let task = rfd::AsyncFileDialog::new().pick_file();
    let ctx = ui.ctx().clone();

    std::thread::spawn(move || {
        futures::executor::block_on(async move {
            let file = task.await;
            // FIXME: Send back an error message when the file can't be opened.
            if let Some(file) = file {
                // FIXME: Send back an error message when the file can't be read.
                let text = file.read().await;
                let _ = sender.send(FileMessage {
                    file: file.path().relative_to(cwd).unwrap(),
                    text: String::from_utf8_lossy(&text).to_string(),
                });
                ctx.request_repaint();
            }
        })
    });
}

fn save_text_to_file(file: &str, text: &str) {
    assert!(!file.is_empty());

    // FIXME: Show a message if the file can't be saved.
    std::fs::write(file, text).expect("Could not save file");
}
