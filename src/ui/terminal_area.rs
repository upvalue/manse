use crate::terminal::TerminalPanel;
use std::collections::BTreeMap;

/// Terminal position information for rendering
pub struct TerminalPosition {
    pub id: u64,
    pub x: f32,
    pub width: f32,
}

/// Calculate terminal positions for a workspace's panel order.
pub fn calculate_positions(
    panel_order: &[u64],
    panels: &BTreeMap<u64, TerminalPanel>,
    viewport_width: f32,
) -> Vec<TerminalPosition> {
    let mut positions = Vec::new();
    let mut x_pos = 0.0;

    for &id in panel_order {
        if let Some(panel) = panels.get(&id) {
            let width = panel.pixel_width(viewport_width);
            positions.push(TerminalPosition {
                id,
                x: x_pos,
                width,
            });
            x_pos += width;
        }
    }

    positions
}

/// Check if a terminal is visible within the viewport.
pub fn is_visible(position: &TerminalPosition, scroll_offset: f32, viewport_width: f32) -> bool {
    let view_left = scroll_offset;
    let view_right = scroll_offset + viewport_width;
    let term_right = position.x + position.width;

    // Terminal is visible if it overlaps with the viewport
    !(term_right < view_left || position.x > view_right)
}

/// Constants for terminal area layout
pub const PADDING: f32 = 4.0;
pub const BORDER_WIDTH: f32 = 2.0;
