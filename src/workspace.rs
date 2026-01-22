/// Cached terminal position data
#[derive(Clone, Default)]
pub struct TerminalPositions {
    /// Cached positions: (panel_id, x_start, width)
    pub positions: Vec<(u64, f32, f32)>,
    /// Viewport width used to compute these positions
    pub viewport_width: f32,
}

/// A workspace containing a horizontal strip of terminals
pub struct Workspace {
    /// Workspace name
    pub name: String,
    /// Order of panels in this workspace (left to right)
    pub panel_order: Vec<u64>,
    /// Currently focused panel index within this workspace
    pub focused_index: usize,
    /// Current scroll offset (animated)
    pub scroll_offset: f32,
    /// Target scroll offset
    pub target_offset: f32,
    /// Cached terminal positions (invalidated when layout changes)
    pub cached_positions: TerminalPositions,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            panel_order: Vec::new(),
            focused_index: 0,
            scroll_offset: 0.0,
            target_offset: 0.0,
            cached_positions: TerminalPositions::default(),
        }
    }

    /// Invalidate cached positions (call when layout changes)
    pub fn invalidate_positions(&mut self) {
        self.cached_positions.viewport_width = 0.0;
    }
}
