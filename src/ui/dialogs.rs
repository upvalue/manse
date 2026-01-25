/// Modal dialog rendering.

use eframe::egui;

/// Result from rendering the confirm close dialog.
pub enum ConfirmCloseResult {
    /// Dialog still open, no action
    None,
    /// User cancelled (escape, background click, or cancel button)
    Cancelled,
    /// User confirmed close
    Confirmed,
}

/// Result from rendering the set description dialog.
pub enum SetDescriptionResult {
    /// Dialog still open with current input
    Open { input: String },
    /// User cancelled
    Cancelled,
    /// User saved with this description
    Saved { description: String },
}

/// Render a semi-transparent background overlay.
fn render_background(ctx: &egui::Context, id: &str) -> bool {
    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();
    let mut clicked = false;

    egui::Area::new(egui::Id::new(id))
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_black_alpha(128),
            );
            if response.clicked() {
                clicked = true;
            }
        });

    clicked
}

/// Render the confirm close terminal dialog.
pub fn render_confirm_close(ctx: &egui::Context) -> ConfirmCloseResult {
    let bg_clicked = render_background(ctx, "dialog_bg");

    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();
    let dialog_width = 300.0;
    let dialog_x = (screen_rect.width() - dialog_width) / 2.0;
    let dialog_y = screen_rect.height() * 0.3;

    let mut should_close = bg_clicked;
    let mut should_confirm = false;

    egui::Area::new(egui::Id::new("confirm_close_dialog"))
        .fixed_pos(egui::pos2(dialog_x, dialog_y))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgb(40, 40, 40))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_width(dialog_width);
                    ui.add_space(16.0);

                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Close Terminal?")
                                .size(16.0)
                                .color(egui::Color32::WHITE),
                        );
                    });

                    ui.add_space(8.0);

                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("This will terminate the running process.")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(160, 160, 160)),
                        );
                    });

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ui.add_space((dialog_width - 160.0) / 2.0);

                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }

                        ui.add_space(8.0);

                        let close_btn = egui::Button::new(
                            egui::RichText::new("Close").color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 60, 60));

                        if ui.add(close_btn).clicked() {
                            should_confirm = true;
                        }
                    });

                    ui.add_space(16.0);
                });
        });

    // Handle keyboard
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        should_close = true;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        should_confirm = true;
    }

    if should_confirm {
        ConfirmCloseResult::Confirmed
    } else if should_close {
        ConfirmCloseResult::Cancelled
    } else {
        ConfirmCloseResult::None
    }
}

/// Render the set description dialog.
pub fn render_set_description(ctx: &egui::Context, current_input: &str) -> SetDescriptionResult {
    let bg_clicked = render_background(ctx, "dialog_bg_desc");

    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();
    let dialog_width = 400.0;
    let dialog_x = (screen_rect.width() - dialog_width) / 2.0;
    let dialog_y = screen_rect.height() * 0.3;

    let mut should_close = bg_clicked;
    let mut should_confirm = false;
    let mut input = current_input.to_string();

    egui::Area::new(egui::Id::new("set_description_dialog"))
        .fixed_pos(egui::pos2(dialog_x, dialog_y))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgb(40, 40, 40))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_width(dialog_width);
                    ui.add_space(16.0);

                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Set Terminal Description")
                                .size(16.0)
                                .color(egui::Color32::WHITE),
                        );
                    });

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        let text_edit = egui::TextEdit::singleline(&mut input)
                            .desired_width(dialog_width - 40.0)
                            .hint_text("Enter description...");
                        let response = ui.add(text_edit);

                        // Always request focus for the text input
                        response.request_focus();

                        // Enter to confirm
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            should_confirm = true;
                        }
                        ui.add_space(16.0);
                    });

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ui.add_space((dialog_width - 160.0) / 2.0);

                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }

                        ui.add_space(8.0);

                        let save_btn = egui::Button::new(
                            egui::RichText::new("Save").color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(60, 120, 180));

                        if ui.add(save_btn).clicked() {
                            should_confirm = true;
                        }
                    });

                    ui.add_space(16.0);
                });
        });

    // Handle escape key
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        should_close = true;
    }

    if should_confirm {
        SetDescriptionResult::Saved { description: input }
    } else if should_close {
        SetDescriptionResult::Cancelled
    } else {
        SetDescriptionResult::Open { input }
    }
}
