use egui::{text::LayoutJob, FontId, TextFormat};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    text: String,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            text: "Test".to_owned(),
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

            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.text)
                            .desired_width(f32::INFINITY)
                            .code_editor()
                            .layouter(&mut |ui: &egui::Ui, text, wrap_width| {
                                let mut job = LayoutJob::default();

                                for (i, word) in text.split(' ').enumerate() {
                                    job.append(
                                        word,
                                        0.0,
                                        TextFormat {
                                            font_id: FontId::new(14.0, egui::FontFamily::Monospace),
                                            color: if i % 2 == 0 {
                                                if ui.ctx().style().visuals.dark_mode {
                                                    egui::Color32::LIGHT_BLUE
                                                } else {
                                                    egui::Color32::BLUE
                                                }
                                            } else {
                                                if ui.ctx().style().visuals.dark_mode {
                                                    egui::Color32::LIGHT_RED
                                                } else {
                                                    egui::Color32::RED
                                                }
                                            },
                                            ..Default::default()
                                        },
                                    );

                                    if i != text.split(' ').count() - 1 {
                                        job.append(
                                            " ",
                                            0.0,
                                            TextFormat {
                                                font_id: FontId::new(
                                                    14.0,
                                                    egui::FontFamily::Monospace,
                                                ),
                                                ..Default::default()
                                            },
                                        );
                                    }
                                }

                                job.wrap.max_width = wrap_width;
                                ui.fonts(|f| f.layout_job(job))
                            }),
                    );
                },
            );
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
