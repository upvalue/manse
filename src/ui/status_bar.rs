use crate::config::{StatusBarConfig, UiConfig};
use crate::terminal::TerminalPanel;
use crate::util::layout::compute_minimap_viewport;
use crate::workspace::Workspace;
use eframe::egui;

/// State needed for rendering the minimap with proportional rectangles.
pub struct MinimapState {
    /// Terminal positions as (x_start, width) pairs
    pub positions: Vec<(f32, f32)>,
    /// Current scroll offset
    pub scroll_offset: f32,
    /// Width of the visible viewport
    pub viewport_width: f32,
}

/// Renders the status bar with terminal indicators and focused terminal info.
pub fn render(
    ui: &mut egui::Ui,
    workspace: &Workspace,
    focused_panel: Option<&TerminalPanel>,
    minimap_state: Option<&MinimapState>,
    config: &StatusBarConfig,
    ui_colors: &UiConfig,
) {
    let num_panels = workspace.panel_order.len();

    ui.horizontal(|ui| {
        ui.add_space(8.0);

        // Left side: Terminal info and title
        ui.label(
            egui::RichText::new(format!("{}/{}", workspace.focused_index + 1, num_panels))
                .size(12.0)
                .color(ui_colors.status_bar_text),
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
                        .color(ui_colors.sidebar_text),
                )
                .truncate(),
            );

            // In-app description (if set via Cmd+D)
            if !panel.description.is_empty() {
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&panel.description)
                            .size(11.0)
                            .color(ui_colors.focused_border),
                    )
                    .truncate(),
                );
            }

            // CLI description (if set via manse term-desc)
            if let Some(ref cli_desc) = panel.cli_description {
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(cli_desc)
                            .size(11.0)
                            .color(ui_colors.status_bar_text),
                    )
                    .truncate(),
                );
            }
        }

        // Right side: Minimap (use remaining space to push to right)
        if config.show_minimap {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);

                // Terminal minimap with fixed-size rectangles
                let minimap_container_width = 160.0;
                let minimap_height = 12.0;
                let rect_gap = 2.0;
                let corner_radius = 2.0;
                // Base width for smallest terminal (1/3 width ratio)
                let base_rect_width = 10.0;
                let min_ratio = 0.333;

                let (response, mut painter) = ui.allocate_painter(
                    egui::vec2(minimap_container_width, 20.0),
                    egui::Sense::hover(),
                );

                let container_rect = response.rect;
                let minimap_y = container_rect.center().y - minimap_height / 2.0;

                // Set clip rect to container bounds
                painter.set_clip_rect(container_rect);

                if let Some(state) = minimap_state {
                    // Calculate fixed widths for each terminal based on ratio
                    let mut term_rects: Vec<(f32, f32, bool)> = Vec::new(); // (x, width, is_focused)
                    let mut x = 0.0;
                    for (i, &(_, term_width)) in state.positions.iter().enumerate() {
                        // Compute width ratio from pixel width
                        let ratio = term_width / state.viewport_width;
                        let rect_width = (ratio / min_ratio) * base_rect_width;
                        term_rects.push((x, rect_width, i == workspace.focused_index));
                        x += rect_width + rect_gap;
                    }
                    let total_content_width = if x > rect_gap { x - rect_gap } else { 0.0 };

                    // Calculate scroll offset to keep focused terminal visible
                    let minimap_scroll = if total_content_width > minimap_container_width {
                        // Find focused terminal position
                        let focused_x = term_rects
                            .iter()
                            .find(|(_, _, focused)| *focused)
                            .map(|(x, w, _)| x + w / 2.0)
                            .unwrap_or(0.0);

                        // Center focused terminal in view, clamped to valid range
                        let target = focused_x - minimap_container_width / 2.0;
                        target.clamp(0.0, total_content_width - minimap_container_width)
                    } else {
                        0.0
                    };

                    // Draw terminal rectangles
                    for (term_x, rect_width, is_focused) in &term_rects {
                        let screen_x = container_rect.left() + term_x - minimap_scroll;

                        let term_rect = egui::Rect::from_min_size(
                            egui::pos2(screen_x, minimap_y),
                            egui::vec2(*rect_width, minimap_height),
                        );

                        let color = if *is_focused {
                            ui_colors.focused_border
                        } else {
                            ui_colors.sidebar_text_dim
                        };

                        painter.rect_filled(term_rect, corner_radius, color);
                    }

                    // Draw viewport indicator
                    if let Some(vp) = compute_minimap_viewport(
                        &state.positions,
                        state.scroll_offset,
                        state.viewport_width,
                    ) {
                        // Scale viewport to fixed-width minimap coordinates
                        let vp_x = vp.x * total_content_width;
                        let vp_width = vp.width * total_content_width;

                        let screen_vp_x = container_rect.left() + vp_x - minimap_scroll;

                        let vp_rect = egui::Rect::from_min_size(
                            egui::pos2(screen_vp_x, minimap_y - 2.0),
                            egui::vec2(vp_width, minimap_height + 4.0),
                        );

                        painter.rect_stroke(
                            vp_rect,
                            corner_radius,
                            egui::Stroke::new(
                                1.5,
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120),
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                } else {
                    // Fallback to simple dots if no minimap state available
                    let dot_radius = 3.0;
                    let dot_spacing = 8.0;
                    let y = container_rect.center().y;

                    for i in 0..num_panels {
                        let x = container_rect.left() + dot_radius + (i as f32 * dot_spacing);
                        let is_active = i == workspace.focused_index;

                        let color = if is_active {
                            ui_colors.focused_border
                        } else {
                            ui_colors.sidebar_text_dim
                        };

                        painter.circle_filled(egui::pos2(x, y), dot_radius, color);
                    }
                }
            });
        }
    });
}
