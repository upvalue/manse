use crate::config::Config;
use crate::terminal::TerminalPanel;
use eframe::egui;
use egui_term::{FontSettings, TerminalFont, TerminalTheme, TerminalView};
use std::collections::HashMap;

pub struct TerminalStripState {
    pub scroll_offset: f32,
    pub focused_index: usize,
    pub positions: Vec<(u64, f32, f32)>,
}

/// Returns the index of the terminal that was clicked, if any
pub fn render(
    ui: &mut egui::Ui,
    config: &Config,
    theme: &TerminalTheme,
    state: &TerminalStripState,
    panels: &mut HashMap<u64, TerminalPanel>,
    dialog_open: bool,
    viewport_width: f32,
    padded_height: f32,
    padding: f32,
) -> Option<usize> {
    let scroll_offset = state.scroll_offset;
    let focused_index = state.focused_index;
    let terminal_positions = &state.positions;

    ui.add_space(padding);

    let border_width = 2.0;
    let terminal_font_size = config.terminal_font_size;

    let view_left = scroll_offset;
    let view_right = scroll_offset + viewport_width;

    let terminal_area = ui.available_rect_before_wrap();
    let base_x = terminal_area.left() + padding;
    let base_y = terminal_area.top();

    let mut clicked_index = None;

    for (idx, &(id, term_x, term_width)) in terminal_positions.iter().enumerate() {
        let term_right = term_x + term_width;

        if term_right < view_left || term_x > view_right {
            continue;
        }

        if let Some(panel) = panels.get_mut(&id) {
            let is_focused = idx == focused_index;

            let screen_x = base_x + term_x - scroll_offset;
            let rect = egui::Rect::from_min_size(
                egui::pos2(screen_x, base_y),
                egui::vec2(term_width, padded_height),
            );

            let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));

            let pad = egui::Margin::symmetric(config.terminal_padding_x as i8, config.terminal_padding_y as i8);
            let base_frame = egui::Frame::NONE
                .inner_margin(pad)
                .fill(config.terminal_background());
            let frame = if is_focused {
                base_frame.stroke(egui::Stroke::new(border_width, config.ui_colors.focused_border))
            } else {
                base_frame
            };

            let inner_width = term_width - border_width * 2.0 - config.terminal_padding_x * 2.0;
            let inner_height = padded_height - border_width * 2.0 - config.terminal_padding_y * 2.0;

            // Check if a primary click happened in this terminal's rect
            let was_clicked = child_ui.input(|i| {
                i.pointer.primary_clicked() && rect.contains(i.pointer.interact_pos().unwrap_or_default())
            });

            if was_clicked {
                clicked_index = Some(idx);
            }

            frame.show(&mut child_ui, |ui| {
                let font = TerminalFont::new(FontSettings {
                    font_type: egui::FontId::monospace(terminal_font_size),
                });
                let term_view = TerminalView::new(ui, &mut panel.backend)
                    .set_focus(is_focused && !dialog_open)
                    .set_font(font)
                    .set_theme(theme.clone())
                    .set_size(egui::vec2(inner_width, inner_height));
                let response = ui.add(term_view);

                if is_focused && !dialog_open {
                    response.request_focus();
                }
            });
        }
    }

    ui.allocate_space(egui::vec2(viewport_width + padding * 2.0, padded_height));

    clicked_index
}
