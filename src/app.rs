use egui::{
    text::LayoutJob, vec2, Align2, Event, EventFilter, FontId, Key, Margin, NumExt, Sense,
    TextFormat, Vec2,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    text: String,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            text: "".to_owned(),
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

            let font_id = FontId::new(14.0, egui::FontFamily::Monospace);

            let layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
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

                job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(job))
            };

            // =============================
            // Calculate dimensions.
            // =============================
            let row_height = ui.fonts(|f| f.row_height(&font_id));

            const MIN_WIDTH: f32 = 24.0;
            let available_width = ui.available_width().at_least(MIN_WIDTH);
            let desired_width = ui.spacing().text_edit_width;
            let wrap_width = if ui.layout().horizontal_justify() {
                available_width
            } else {
                desired_width.min(available_width)
            };

            let galley = layouter(ui, &self.text, wrap_width);

            // No clipping, always wrap for now.
            let desired_width = galley.size().x.max(wrap_width);
            // Desired height is one row of text for now.
            let desired_height = 4.0 * row_height;
            // Default values form the TextGui TextEdit.
            let at_least = Vec2::ZERO - Margin::symmetric(4.0, 2.0).sum();
            let desired_size =
                vec2(desired_width, galley.size().y.max(desired_height)).at_least(at_least);

            let (id, rect) = ui.allocate_space(desired_size);

            let painter = ui.painter_at(rect.expand(1.0)); // expand to avoid clipping cursor.

            let galley_pos = Align2::LEFT_TOP
                .align_size_within_rect(galley.size(), rect)
                .intersect(rect) // limit pos to the response rect area
                .min;

            // =============================
            // Do interactions.
            // =============================
            let sense = Sense::click_and_drag();
            let response = ui.interact(rect, id, sense);

            if let Some(_pointer_pos) = ui.ctx().pointer_interact_pos() {
                if response.hovered() {
                    ui.output_mut(|o| o.mutable_text_under_cursor = true);
                }
            }

            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }

            if response.clicked() {
                ui.memory_mut(|m| m.request_focus(response.id));
            }

            let event_filter = EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                tab: true,
                ..Default::default()
            };

            if ui.memory(|m| m.has_focus(id)) {
                let events = ui.input(|i| i.filtered_events(&event_filter));

                ui.memory_mut(|m| m.set_focus_lock_filter(id, event_filter));

                for event in &events {
                    match event {
                        Event::Text(text_to_insert) => {
                            if !text_to_insert.is_empty()
                                && text_to_insert != "\n"
                                && text_to_insert != "\r"
                            {
                                self.text += text_to_insert;
                            }
                        }
                        Event::Key {
                            key: Key::Tab,
                            pressed: true,
                            ..
                        } => {
                            self.text += "\t";
                        }
                        Event::Key {
                            key: Key::Enter,
                            pressed: true,
                            ..
                        } => {
                            self.text += "\n";
                        }
                        Event::Key {
                            key: Key::Backspace,
                            pressed: true,
                            ..
                        } => {
                            self.text.pop();
                        }
                        _ => (),
                    }
                }
            }

            // =============================
            // Draw the text.
            // =============================
            if ui.is_rect_visible(rect) {
                painter.galley(galley_pos, galley.clone(), egui::Color32::WHITE);
            }
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
