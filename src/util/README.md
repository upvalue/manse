# Utility Module

Pure, side-effect-free functions that are easy to unit test.

## Design Principles

- **No I/O**: Functions don't read files, make network calls, or interact with the OS
- **No global state**: All inputs are explicit parameters
- **No framework dependencies**: No egui, no async runtimes
- **Deterministic**: Same inputs always produce same outputs (except `ids` which uses randomness)

## Modules

### `layout.rs` - Layout and Scroll Math

Calculations for the scrolling window manager:

- `compute_positions()` - Calculate x positions from panel widths
- `total_width()` - Sum total content width
- `scroll_target_for_visible()` - Calculate scroll offset to show a terminal
- `ease_toward()` - Smooth scroll animation easing
- `is_animating()` - Check if animation is in progress
- `visible_range()` - Determine which panels are visible in viewport
- `build_follow_targets()` - Map flat index to (workspace, terminal) coordinates
- `index_to_letter()` / `letter_to_index()` - Convert between 0-25 and a-z
- `next_ratio()` / `prev_ratio()` - Step through width ratios

### `ids.rs` - ID Generation

Terminal identifier generation:

- `new_terminal_id()` - Generate unique "term-XXXXXXXXXXXX" IDs
- `is_valid_terminal_id()` - Validate ID format

### `icons.rs` - Icon Detection

Map terminal titles to display icons:

- `detect_icon()` - Pattern match title to icon (claude, nvim, vim, etc.)

## Testing

Run all util tests:

```bash
cargo test util::
```

Current coverage: 48 tests
