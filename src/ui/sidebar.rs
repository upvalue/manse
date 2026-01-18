use crate::terminal::TerminalPanel;
use crate::workspace::Workspace;
use eframe::egui;
use std::collections::BTreeMap;

/// Renders the sidebar with workspace and terminal list.
/// Returns Some((workspace_idx, terminal_idx)) if a terminal was clicked.
pub fn render(
    ui: &mut egui::Ui,
    workspaces: &[Workspace],
    active_workspace: usize,
    panels: &BTreeMap<u64, TerminalPanel>,
) -> Option<(usize, usize)> {
    ui.vertical_centered(|ui| {
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new("Manse")
                .size(16.0)
                .color(egui::Color32::from_rgb(150, 150, 150)),
        );
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    let mut clicked_terminal: Option<(usize, usize)> = None;

    for (ws_idx, ws) in workspaces.iter().enumerate() {
        let is_active_workspace = ws_idx == active_workspace;

        // Workspace name
        let ws_color = if is_active_workspace {
            egui::Color32::from_rgb(200, 200, 200)
        } else {
            egui::Color32::from_rgb(120, 120, 120)
        };

        ui.label(egui::RichText::new(&ws.name).size(13.0).color(ws_color));

        // Terminals in this workspace (indented)
        ui.indent(ws_idx, |ui| {
            for (term_idx, &id) in ws.panel_order.iter().enumerate() {
                if let Some(panel) = panels.get(&id) {
                    let is_focused = is_active_workspace && term_idx == ws.focused_index;
                    let text_color = if is_focused {
                        egui::Color32::from_rgb(100, 150, 255)
                    } else {
                        egui::Color32::from_rgb(180, 180, 180)
                    };

                    // Title
                    let response = ui.add(
                        egui::Label::new(
                            egui::RichText::new(panel.display_title())
                                .size(12.0)
                                .color(text_color),
                        )
                        .truncate()
                        .sense(egui::Sense::click()),
                    );

                    if response.clicked() {
                        clicked_terminal = Some((ws_idx, term_idx));
                    }

                    // Description (if set)
                    if !panel.description.is_empty() {
                        let desc_color = if is_focused {
                            egui::Color32::from_rgb(80, 120, 200)
                        } else {
                            egui::Color32::from_rgb(120, 120, 120)
                        };
                        let desc_response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&panel.description)
                                    .size(10.0)
                                    .color(desc_color),
                            )
                            .truncate()
                            .sense(egui::Sense::click()),
                        );
                        if desc_response.clicked() {
                            clicked_terminal = Some((ws_idx, term_idx));
                        }
                    }
                }
            }
        });

        ui.add_space(4.0);
    }

    clicked_terminal
}
