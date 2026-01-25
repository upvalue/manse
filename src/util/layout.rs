/// Pure layout and scroll math functions.
///
/// These functions have no dependencies on application state and are easily unit tested.

/// Scroll animation easing factor
pub const SCROLL_EASING: f32 = 0.15;

/// Compute (x_position, width) for each panel given their widths.
pub fn compute_positions(panel_widths: impl Iterator<Item = f32>) -> Vec<(f32, f32)> {
    let mut positions = Vec::new();
    let mut x = 0.0;
    for width in panel_widths {
        positions.push((x, width));
        x += width;
    }
    positions
}

/// Total content width from position list.
pub fn total_width(positions: &[(f32, f32)]) -> f32 {
    positions.last().map(|(x, w)| x + w).unwrap_or(0.0)
}

/// Calculate scroll target to keep a terminal visible in viewport.
///
/// Returns a new target scroll offset that ensures the terminal at `index`
/// is fully visible within the viewport.
pub fn scroll_target_for_visible(
    positions: &[(f32, f32)],
    index: usize,
    current_target: f32,
    viewport_width: f32,
) -> f32 {
    let Some(&(term_x, term_width)) = positions.get(index) else {
        return current_target;
    };

    let term_right = term_x + term_width;
    let view_right = current_target + viewport_width;

    let mut target = current_target;

    // If terminal's left edge is off-screen left, scroll left
    if term_x < current_target {
        target = term_x;
    }
    // If terminal's right edge is off-screen right, scroll right
    else if term_right > view_right {
        target = term_right - viewport_width;
    }

    // Clamp to valid scroll range
    let max_scroll = (total_width(positions) - viewport_width).max(0.0);
    target.clamp(0.0, max_scroll)
}

/// Apply easing to animate scroll offset toward target.
///
/// When the difference is small (< 0.5 pixels), snaps directly to target
/// to avoid endless tiny animations.
pub fn ease_toward(current: f32, target: f32, easing: f32) -> f32 {
    let diff = target - current;
    if diff.abs() > 0.5 {
        current + diff * easing
    } else {
        target
    }
}

/// Check if scroll animation is still in progress.
pub fn is_animating(current: f32, target: f32) -> bool {
    (current - target).abs() > 0.5
}

/// Determine which panels are visible in the current viewport.
///
/// Returns indices of panels that are at least partially visible.
pub fn visible_range(
    positions: &[(f32, f32)],
    scroll_offset: f32,
    viewport_width: f32,
) -> impl Iterator<Item = usize> + '_ {
    let view_left = scroll_offset;
    let view_right = scroll_offset + viewport_width;

    positions
        .iter()
        .enumerate()
        .filter(move |(_, (x, w))| {
            let right = x + w;
            // Panel is visible if it overlaps with viewport
            right > view_left && *x < view_right
        })
        .map(|(i, _)| i)
}

/// Maximum number of follow mode targets (a-z)
pub const MAX_FOLLOW_TARGETS: usize = 26;

/// Build a flat mapping of index (0-25) to (workspace_idx, terminal_idx).
///
/// Given a list of terminal counts per workspace, returns coordinates for
/// follow mode navigation. Limited to 26 entries (a-z).
pub fn build_follow_targets(workspace_terminal_counts: &[usize]) -> Vec<(usize, usize)> {
    let mut targets = Vec::new();
    for (ws_idx, &count) in workspace_terminal_counts.iter().enumerate() {
        for term_idx in 0..count {
            if targets.len() >= MAX_FOLLOW_TARGETS {
                return targets;
            }
            targets.push((ws_idx, term_idx));
        }
    }
    targets
}

/// Convert a letter index (0=a, 1=b, ..., 25=z) to its character.
pub fn index_to_letter(index: usize) -> Option<char> {
    if index < 26 {
        Some((b'a' + index as u8) as char)
    } else {
        None
    }
}

/// Convert a letter character to its index (a=0, b=1, ..., z=25).
pub fn letter_to_index(letter: char) -> Option<usize> {
    let lower = letter.to_ascii_lowercase();
    if lower.is_ascii_lowercase() {
        Some((lower as u8 - b'a') as usize)
    } else {
        None
    }
}

/// Find the next larger value in a sorted list of ratios.
///
/// Returns the first ratio that is greater than `current` by at least `epsilon`.
/// Returns `None` if `current` is already at or above the maximum.
pub fn next_ratio(ratios: &[f32], current: f32, epsilon: f32) -> Option<f32> {
    ratios.iter().find(|&&r| r > current + epsilon).copied()
}

/// Find the next smaller value in a sorted list of ratios.
///
/// Returns the last ratio that is less than `current` by at least `epsilon`.
/// Returns `None` if `current` is already at or below the minimum.
pub fn prev_ratio(ratios: &[f32], current: f32, epsilon: f32) -> Option<f32> {
    ratios.iter().rev().find(|&&r| r < current - epsilon).copied()
}

/// Minimap rectangle for a single terminal.
#[derive(Debug, Clone, PartialEq)]
pub struct MinimapRect {
    /// X offset in minimap coordinates (0.0 to 1.0 normalized)
    pub x: f32,
    /// Width in minimap coordinates (0.0 to 1.0 normalized)
    pub width: f32,
}

/// Viewport rectangle in minimap coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct MinimapViewport {
    /// X offset in minimap coordinates (0.0 to 1.0 normalized)
    pub x: f32,
    /// Width in minimap coordinates (0.0 to 1.0 normalized)
    pub width: f32,
}

/// Compute minimap rectangles from terminal positions.
///
/// Returns normalized coordinates (0.0 to 1.0) representing each terminal's
/// position and width relative to the total content width.
///
/// # Arguments
/// * `positions` - Terminal positions as (x_start, width) pairs
///
/// # Returns
/// Vector of `MinimapRect` with normalized x and width values
pub fn compute_minimap_rects(positions: &[(f32, f32)]) -> Vec<MinimapRect> {
    let total = total_width(positions);
    if total <= 0.0 {
        return Vec::new();
    }

    positions
        .iter()
        .map(|(x, w)| MinimapRect {
            x: x / total,
            width: w / total,
        })
        .collect()
}

/// Compute viewport rectangle in minimap coordinates.
///
/// Returns the viewport's position and width normalized to (0.0 to 1.0),
/// representing what portion of the total content is currently visible.
///
/// # Arguments
/// * `positions` - Terminal positions as (x_start, width) pairs
/// * `scroll_offset` - Current scroll position
/// * `viewport_width` - Width of the visible viewport
///
/// # Returns
/// `MinimapViewport` with normalized x and width, or None if no content
pub fn compute_minimap_viewport(
    positions: &[(f32, f32)],
    scroll_offset: f32,
    viewport_width: f32,
) -> Option<MinimapViewport> {
    let total = total_width(positions);
    if total <= 0.0 {
        return None;
    }

    // Clamp scroll offset to valid range
    let max_scroll = (total - viewport_width).max(0.0);
    let clamped_offset = scroll_offset.clamp(0.0, max_scroll);

    // Viewport width can't exceed total content
    let effective_viewport = viewport_width.min(total);

    Some(MinimapViewport {
        x: clamped_offset / total,
        width: effective_viewport / total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_positions_empty() {
        let positions = compute_positions(std::iter::empty());
        assert!(positions.is_empty());
    }

    #[test]
    fn compute_positions_single() {
        let positions = compute_positions([100.0].into_iter());
        assert_eq!(positions, vec![(0.0, 100.0)]);
    }

    #[test]
    fn compute_positions_multiple() {
        let widths = [100.0, 200.0, 150.0];
        let positions = compute_positions(widths.into_iter());
        assert_eq!(
            positions,
            vec![(0.0, 100.0), (100.0, 200.0), (300.0, 150.0)]
        );
    }

    #[test]
    fn total_width_empty() {
        assert_eq!(total_width(&[]), 0.0);
    }

    #[test]
    fn total_width_single() {
        assert_eq!(total_width(&[(0.0, 100.0)]), 100.0);
    }

    #[test]
    fn total_width_multiple() {
        let positions = vec![(0.0, 100.0), (100.0, 200.0), (300.0, 150.0)];
        assert_eq!(total_width(&positions), 450.0);
    }

    #[test]
    fn scroll_target_already_visible() {
        let positions = vec![(0.0, 100.0), (100.0, 100.0), (200.0, 100.0)];
        // Terminal 1 (x=100, w=100) is fully visible in viewport 0..300
        let target = scroll_target_for_visible(&positions, 1, 0.0, 300.0);
        assert_eq!(target, 0.0);
    }

    #[test]
    fn scroll_target_off_right() {
        let positions = vec![(0.0, 200.0), (200.0, 200.0), (400.0, 200.0)];
        // Terminal 2 (x=400, w=200) is off-screen right, viewport=300
        let target = scroll_target_for_visible(&positions, 2, 0.0, 300.0);
        // Should scroll so right edge (600) aligns with viewport right: 600 - 300 = 300
        assert_eq!(target, 300.0);
    }

    #[test]
    fn scroll_target_off_left() {
        let positions = vec![(0.0, 200.0), (200.0, 200.0), (400.0, 200.0)];
        // Terminal 0 is off-screen left when scrolled to 300
        let target = scroll_target_for_visible(&positions, 0, 300.0, 300.0);
        assert_eq!(target, 0.0);
    }

    #[test]
    fn scroll_target_clamps_to_max() {
        let positions = vec![(0.0, 100.0)];
        // Content (100) is smaller than viewport (300), should clamp to 0
        let target = scroll_target_for_visible(&positions, 0, 50.0, 300.0);
        assert_eq!(target, 0.0);
    }

    #[test]
    fn scroll_target_invalid_index() {
        let positions = vec![(0.0, 100.0)];
        // Invalid index should return current target unchanged
        let target = scroll_target_for_visible(&positions, 5, 50.0, 300.0);
        assert_eq!(target, 50.0);
    }

    #[test]
    fn ease_toward_snaps_when_close() {
        assert_eq!(ease_toward(99.8, 100.0, 0.15), 100.0);
        assert_eq!(ease_toward(100.3, 100.0, 0.15), 100.0);
    }

    #[test]
    fn ease_toward_moves_partially() {
        let result = ease_toward(0.0, 100.0, 0.15);
        assert!((result - 15.0).abs() < 0.001);
    }

    #[test]
    fn ease_toward_negative_direction() {
        let result = ease_toward(100.0, 0.0, 0.15);
        assert!((result - 85.0).abs() < 0.001);
    }

    #[test]
    fn is_animating_true_when_far() {
        assert!(is_animating(0.0, 100.0));
        assert!(is_animating(100.0, 0.0));
    }

    #[test]
    fn is_animating_false_when_close() {
        assert!(!is_animating(99.8, 100.0));
        assert!(!is_animating(100.0, 100.0));
    }

    #[test]
    fn visible_range_all_visible() {
        let positions = vec![(0.0, 100.0), (100.0, 100.0), (200.0, 100.0)];
        let visible: Vec<_> = visible_range(&positions, 0.0, 400.0).collect();
        assert_eq!(visible, vec![0, 1, 2]);
    }

    #[test]
    fn visible_range_partial() {
        let positions = vec![(0.0, 100.0), (100.0, 100.0), (200.0, 100.0), (300.0, 100.0)];
        // Viewport from 50 to 250 should see panels 0, 1, 2
        let visible: Vec<_> = visible_range(&positions, 50.0, 200.0).collect();
        assert_eq!(visible, vec![0, 1, 2]);
    }

    #[test]
    fn visible_range_scrolled() {
        let positions = vec![(0.0, 100.0), (100.0, 100.0), (200.0, 100.0), (300.0, 100.0)];
        // Viewport from 200 to 400 should see panels 2, 3
        let visible: Vec<_> = visible_range(&positions, 200.0, 200.0).collect();
        assert_eq!(visible, vec![2, 3]);
    }

    #[test]
    fn visible_range_empty() {
        let positions: Vec<(f32, f32)> = vec![];
        let visible: Vec<_> = visible_range(&positions, 0.0, 100.0).collect();
        assert!(visible.is_empty());
    }

    #[test]
    fn follow_targets_empty() {
        let targets = build_follow_targets(&[]);
        assert!(targets.is_empty());
    }

    #[test]
    fn follow_targets_single_workspace() {
        let targets = build_follow_targets(&[3]);
        assert_eq!(targets, vec![(0, 0), (0, 1), (0, 2)]);
    }

    #[test]
    fn follow_targets_multiple_workspaces() {
        let targets = build_follow_targets(&[2, 3, 1]);
        assert_eq!(
            targets,
            vec![(0, 0), (0, 1), (1, 0), (1, 1), (1, 2), (2, 0)]
        );
    }

    #[test]
    fn follow_targets_caps_at_26() {
        // 30 terminals should be capped at 26
        let targets = build_follow_targets(&[30]);
        assert_eq!(targets.len(), 26);
        assert_eq!(targets[0], (0, 0));
        assert_eq!(targets[25], (0, 25));
    }

    #[test]
    fn follow_targets_caps_across_workspaces() {
        // 10 + 10 + 10 = 30, should cap at 26
        let targets = build_follow_targets(&[10, 10, 10]);
        assert_eq!(targets.len(), 26);
        // First 10 from ws 0, next 10 from ws 1, only 6 from ws 2
        assert_eq!(targets[9], (0, 9));
        assert_eq!(targets[10], (1, 0));
        assert_eq!(targets[19], (1, 9));
        assert_eq!(targets[20], (2, 0));
        assert_eq!(targets[25], (2, 5));
    }

    #[test]
    fn index_to_letter_valid() {
        assert_eq!(index_to_letter(0), Some('a'));
        assert_eq!(index_to_letter(25), Some('z'));
        assert_eq!(index_to_letter(12), Some('m'));
    }

    #[test]
    fn index_to_letter_invalid() {
        assert_eq!(index_to_letter(26), None);
        assert_eq!(index_to_letter(100), None);
    }

    #[test]
    fn letter_to_index_valid() {
        assert_eq!(letter_to_index('a'), Some(0));
        assert_eq!(letter_to_index('z'), Some(25));
        assert_eq!(letter_to_index('A'), Some(0)); // case insensitive
        assert_eq!(letter_to_index('Z'), Some(25));
    }

    #[test]
    fn letter_to_index_invalid() {
        assert_eq!(letter_to_index('0'), None);
        assert_eq!(letter_to_index('!'), None);
    }

    #[test]
    fn next_ratio_finds_larger() {
        let ratios = [0.333, 0.5, 0.667, 1.0];
        assert_eq!(next_ratio(&ratios, 0.333, 0.01), Some(0.5));
        assert_eq!(next_ratio(&ratios, 0.5, 0.01), Some(0.667));
        assert_eq!(next_ratio(&ratios, 0.667, 0.01), Some(1.0));
    }

    #[test]
    fn next_ratio_none_at_max() {
        let ratios = [0.333, 0.5, 0.667, 1.0];
        assert_eq!(next_ratio(&ratios, 1.0, 0.01), None);
        assert_eq!(next_ratio(&ratios, 0.995, 0.01), None); // within epsilon of max
    }

    #[test]
    fn next_ratio_from_below() {
        let ratios = [0.333, 0.5, 0.667, 1.0];
        // If current is 0.4, next is 0.5 (which is > 0.4 + 0.01)
        assert_eq!(next_ratio(&ratios, 0.4, 0.01), Some(0.5));
        // If current is 0.49, next is 0.667 (0.5 is NOT > 0.49 + 0.01 = 0.50)
        assert_eq!(next_ratio(&ratios, 0.49, 0.01), Some(0.667));
    }

    #[test]
    fn prev_ratio_finds_smaller() {
        let ratios = [0.333, 0.5, 0.667, 1.0];
        assert_eq!(prev_ratio(&ratios, 1.0, 0.01), Some(0.667));
        assert_eq!(prev_ratio(&ratios, 0.667, 0.01), Some(0.5));
        assert_eq!(prev_ratio(&ratios, 0.5, 0.01), Some(0.333));
    }

    #[test]
    fn prev_ratio_none_at_min() {
        let ratios = [0.333, 0.5, 0.667, 1.0];
        assert_eq!(prev_ratio(&ratios, 0.333, 0.01), None);
        assert_eq!(prev_ratio(&ratios, 0.34, 0.01), None); // within epsilon of min
    }

    // Minimap tests

    #[test]
    fn minimap_rects_empty() {
        let rects = compute_minimap_rects(&[]);
        assert!(rects.is_empty());
    }

    #[test]
    fn minimap_rects_single() {
        let positions = vec![(0.0, 100.0)];
        let rects = compute_minimap_rects(&positions);
        assert_eq!(rects.len(), 1);
        assert!((rects[0].x - 0.0).abs() < 0.001);
        assert!((rects[0].width - 1.0).abs() < 0.001);
    }

    #[test]
    fn minimap_rects_equal_widths() {
        // Three equal-width terminals
        let positions = vec![(0.0, 100.0), (100.0, 100.0), (200.0, 100.0)];
        let rects = compute_minimap_rects(&positions);
        assert_eq!(rects.len(), 3);
        // Each should be 1/3 wide
        for (i, rect) in rects.iter().enumerate() {
            let expected_x = i as f32 / 3.0;
            assert!((rect.x - expected_x).abs() < 0.001);
            assert!((rect.width - 1.0 / 3.0).abs() < 0.001);
        }
    }

    #[test]
    fn minimap_rects_variable_widths() {
        // Terminals: 1/4, 1/2, 1/4 of total
        let positions = vec![(0.0, 100.0), (100.0, 200.0), (300.0, 100.0)];
        let rects = compute_minimap_rects(&positions);
        assert_eq!(rects.len(), 3);
        // First: x=0, width=0.25
        assert!((rects[0].x - 0.0).abs() < 0.001);
        assert!((rects[0].width - 0.25).abs() < 0.001);
        // Second: x=0.25, width=0.5
        assert!((rects[1].x - 0.25).abs() < 0.001);
        assert!((rects[1].width - 0.5).abs() < 0.001);
        // Third: x=0.75, width=0.25
        assert!((rects[2].x - 0.75).abs() < 0.001);
        assert!((rects[2].width - 0.25).abs() < 0.001);
    }

    #[test]
    fn minimap_viewport_empty() {
        let viewport = compute_minimap_viewport(&[], 0.0, 100.0);
        assert!(viewport.is_none());
    }

    #[test]
    fn minimap_viewport_single_fits() {
        // Single terminal that fits entirely in viewport
        let positions = vec![(0.0, 100.0)];
        let viewport = compute_minimap_viewport(&positions, 0.0, 200.0).unwrap();
        // Viewport covers entire content
        assert!((viewport.x - 0.0).abs() < 0.001);
        assert!((viewport.width - 1.0).abs() < 0.001);
    }

    #[test]
    fn minimap_viewport_scrolled() {
        // Content: 400px total, viewport: 200px, scrolled to middle
        let positions = vec![(0.0, 200.0), (200.0, 200.0)];
        let viewport = compute_minimap_viewport(&positions, 100.0, 200.0).unwrap();
        // Viewport at 25% offset (100/400), width 50% (200/400)
        assert!((viewport.x - 0.25).abs() < 0.001);
        assert!((viewport.width - 0.5).abs() < 0.001);
    }

    #[test]
    fn minimap_viewport_at_end() {
        // Scrolled to end
        let positions = vec![(0.0, 200.0), (200.0, 200.0)];
        let viewport = compute_minimap_viewport(&positions, 200.0, 200.0).unwrap();
        // Viewport at 50% offset, width 50%
        assert!((viewport.x - 0.5).abs() < 0.001);
        assert!((viewport.width - 0.5).abs() < 0.001);
    }

    #[test]
    fn minimap_viewport_clamps_scroll() {
        // Scroll offset exceeds max
        let positions = vec![(0.0, 100.0)];
        let viewport = compute_minimap_viewport(&positions, 500.0, 50.0).unwrap();
        // Should clamp to max scroll (100 - 50 = 50)
        assert!((viewport.x - 0.5).abs() < 0.001);
        assert!((viewport.width - 0.5).abs() < 0.001);
    }
}
