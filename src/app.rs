use egui::{
    text::LayoutJob, vec2, Align2, Event, EventFilter, FontId, Key, Margin, NumExt, Sense, Shape,
    TextFormat, Vec2,
};
use epaint::text::cursor::Cursor;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    text: String,

    #[serde(skip)]
    cursor: Cursor,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            text: "".to_owned(),
            cursor: Cursor::default(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
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

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
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
                    let mut job = LayoutJob::default();

                    for (i, word) in text.split(' ').enumerate() {
                        job.append(
                            word,
                            0.0,
                            TextFormat {
                                font_id: font_id.clone(),
                                color: if i % 2 == 0 {
                                    if ui.ctx().style().visuals.dark_mode {
                                        egui::Color32::LIGHT_BLUE
                                    } else {
                                        egui::Color32::BLUE
                                    }
                                } else if ui.ctx().style().visuals.dark_mode {
                                    egui::Color32::LIGHT_RED
                                } else {
                                    egui::Color32::RED
                                },
                                ..Default::default()
                            },
                        );

                        if i != text.split(' ').count() - 1 {
                            job.append(
                                " ",
                                0.0,
                                TextFormat {
                                    font_id: FontId::new(14.0, egui::FontFamily::Monospace),
                                    ..Default::default()
                                },
                            );
                        }
                    }

                    job.wrap.max_width = f32::INFINITY;
                    ui.fonts(|f| f.layout_job(job))
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

                // =============================
                // Do interactions.
                // =============================
                let sense = Sense::click_and_drag();
                let mut response = content_ui.interact(rect, id, sense);

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
                        let new_cursor = match event {
                            Event::Text(text_to_insert) => {
                                if !text_to_insert.is_empty()
                                    && text_to_insert != "\n"
                                    && text_to_insert != "\r"
                                {
                                    self.text += text_to_insert;

                                    Some(self.cursor.ccursor + 1)
                                } else {
                                    None
                                }
                            }
                            Event::Key {
                                key: Key::Tab,
                                pressed: true,
                                ..
                            } => {
                                self.text += "\t";

                                Some(self.cursor.ccursor + 1)
                            }
                            Event::Key {
                                key: Key::Enter,
                                pressed: true,
                                ..
                            } => {
                                self.text += "\n";

                                Some(self.cursor.ccursor + 1)
                            }
                            Event::Key {
                                key: Key::Backspace,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                let ccursor = if modifiers.command || modifiers.mac_cmd {
                                    self.text.clear();

                                    Cursor::default().ccursor
                                } else {
                                    self.text.pop();

                                    self.cursor.ccursor - 1
                                };

                                Some(ccursor)
                            }
                            _ => None,
                        };

                        if let Some(new_cursor) = new_cursor {
                            galley = layouter(&content_ui, &self.text, wrap_width);
                            self.cursor = galley.from_ccursor(new_cursor);
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

                // FIXME: support multiple cursors/selections.
                let mut cursor_pos = galley
                    .pos_from_cursor(&self.cursor)
                    .translate(galley_pos.to_vec2());

                // Handle completely empty galleys
                cursor_pos.max.y = cursor_pos.max.y.at_least(cursor_pos.min.y + row_height);
                // Expand to slightly above and below the text.
                cursor_pos = cursor_pos.expand(1.5);

                let cursor_stroke = ui.visuals().text_cursor;
                let top = cursor_pos.center_top();
                let bottom = cursor_pos.center_bottom();

                painter.line_segment([top, bottom], (cursor_stroke.width, cursor_stroke.color));

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
