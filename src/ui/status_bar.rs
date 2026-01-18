use crate::terminal::TerminalPanel;
use crate::workspace::Workspace;
use eframe::egui;

/// Renders the status bar with terminal indicators and focused terminal info.
pub fn render(ui: &mut egui::Ui, workspace: &Workspace, focused_panel: Option<&TerminalPanel>) {
    let num_panels = workspace.panel_order.len();

    ui.horizontal(|ui| {
        ui.add_space(8.0);

        // Terminal minimap dots
        let dot_radius = 4.0;
        let dot_spacing = 12.0;

        let (response, painter) = ui.allocate_painter(
            egui::vec2(num_panels as f32 * dot_spacing + dot_radius * 2.0, 20.0),
            egui::Sense::hover(),
        );

        let rect = response.rect;
        let y = rect.center().y;

        for i in 0..num_panels {
            let x = rect.left() + dot_radius + (i as f32 * dot_spacing);
            let is_active = i == workspace.focused_index;

            let color = if is_active {
                egui::Color32::from_rgb(100, 150, 255)
            } else {
                egui::Color32::from_rgb(80, 80, 80)
            };

            painter.circle_filled(egui::pos2(x, y), dot_radius, color);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Terminal info
        ui.label(
            egui::RichText::new(format!("{}/{}", workspace.focused_index + 1, num_panels))
                .size(12.0)
                .color(egui::Color32::from_rgb(120, 120, 120)),
        );

        // Focused terminal title and description
        if let Some(panel) = focused_panel {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.add(
                egui::Label::new(
                    egui::RichText::new(panel.display_title())
                        .size(12.0)
                        .color(egui::Color32::from_rgb(180, 180, 180)),
                )
                .truncate(),
            );

            // Description (if set)
            if !panel.description.is_empty() {
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&panel.description)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(120, 120, 120)),
                    )
                    .truncate(),
                );
            }
        }
    });
}
