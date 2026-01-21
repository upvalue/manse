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
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            panel_order: Vec::new(),
            focused_index: 0,
            scroll_offset: 0.0,
            target_offset: 0.0,
        }
    }
}
