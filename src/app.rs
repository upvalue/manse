use crate::command::Command;
use crate::config::Config;
use crate::ipc::{start_ipc_server, IpcHandle, Request, Response};
use crate::persist::{self, PersistedState, PersistedTerminal, PersistedWorkspace};
use crate::terminal::TerminalPanel;
use crate::ui::{command_palette, sidebar, status_bar};
use crate::workspace::Workspace;
use eframe::egui;
use egui_term::{FontSettings, PtyEvent, TerminalFont, TerminalView};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance tracking for debugging battery/CPU usage
#[derive(Default)]
struct PerfStats {
    /// When the current measurement window started
    window_start: Option<Instant>,
    /// Number of frames rendered in this window
    frame_count: u64,
    /// Number of PTY events processed
    pty_events: u64,
    /// Number of IPC requests processed
    ipc_requests: u64,
    /// Frames where scroll animation was active
    scroll_animation_frames: u64,
    /// Frames while minimized
    minimized_frames: u64,
    /// Frames with pointer (mouse) activity
    pointer_frames: u64,
    /// Frames with keyboard activity
    keyboard_frames: u64,
    /// Frames where window has focus
    focused_frames: u64,
}

/// Font data
const JETBRAINS_MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const SYMBOLS_NERD_BYTES: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");
const NOTO_SYMBOLS_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols-Regular.ttf");
const NOTO_SYMBOLS2_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
const NOTO_EMOJI_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf");

/// Configure fonts: JetBrains Mono primary, Nerd Font + Noto Emoji fallbacks
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Add font data
    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        Arc::new(egui::FontData::from_static(JETBRAINS_MONO_BYTES)),
    );
    fonts.font_data.insert(
        "symbols_nerd".to_owned(),
        Arc::new(egui::FontData::from_static(SYMBOLS_NERD_BYTES)),
    );
    fonts.font_data.insert(
        "noto_symbols".to_owned(),
        Arc::new(egui::FontData::from_static(NOTO_SYMBOLS_BYTES)),
    );
    fonts.font_data.insert(
        "noto_symbols2".to_owned(),
        Arc::new(egui::FontData::from_static(NOTO_SYMBOLS2_BYTES)),
    );
    fonts.font_data.insert(
        "noto_emoji".to_owned(),
        Arc::new(egui::FontData::from_static(NOTO_EMOJI_BYTES)),
    );

    // Monospace: JetBrains Mono first, then Nerd Font, then Symbols, then Emoji
    let mono = fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap();
    mono.insert(0, "jetbrains_mono".to_owned());
    mono.push("symbols_nerd".to_owned());
    mono.push("noto_symbols".to_owned());
    mono.push("noto_symbols2".to_owned());
    mono.push("noto_emoji".to_owned());

    // Proportional: keep defaults but add fallbacks
    let prop = fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap();
    prop.push("symbols_nerd".to_owned());
    prop.push("noto_symbols".to_owned());
    prop.push("noto_symbols2".to_owned());
    prop.push("noto_emoji".to_owned());

    ctx.set_fonts(fonts);

    // Install image loaders for PNG support
    egui_extras::install_image_loaders(ctx);
}

/// Width ratios for terminal panels
pub const WIDTH_RATIOS: [f32; 4] = [0.333, 0.5, 0.667, 1.0];

/// Scroll animation easing factor
const SCROLL_EASING: f32 = 0.15;

/// Active dialog type
#[derive(Default)]
enum ActiveDialog {
    #[default]
    None,
    /// Confirm close terminal dialog
    ConfirmClose,
    /// Set description input dialog
    SetDescription {
        input: String,
    },
}

/// The scrolling window manager
pub struct App {
    /// Application configuration
    config: Config,
    /// Terminal panels (global pool)
    panels: HashMap<u64, TerminalPanel>,
    /// Workspaces
    workspaces: Vec<Workspace>,
    /// Currently active workspace index
    active_workspace: usize,
    /// Next panel ID
    next_id: u64,
    /// Event receiver for PTY events
    event_rx: Receiver<(u64, PtyEvent)>,
    /// Event sender for creating new terminals
    event_tx: Sender<(u64, PtyEvent)>,
    /// IPC handle for external control (server runs in background thread)
    ipc_handle: Option<IpcHandle>,
    /// Socket path for IPC (passed to terminal env)
    socket_path: Option<PathBuf>,
    /// Whether the command palette is open
    command_palette_open: bool,
    /// Whether follow mode is active (jump to terminal by letter)
    follow_mode: bool,
    /// Performance tracking stats
    perf_stats: PerfStats,
    /// Active dialog (confirmation, input, etc.)
    active_dialog: ActiveDialog,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, socket_path: Option<PathBuf>, config: Config) -> Self {
        // Configure fonts with emoji support
        setup_fonts(&cc.egui_ctx);

        let (event_tx, event_rx) = mpsc::channel();

        // Initialize IPC server in background thread if socket path provided
        let ipc_handle = socket_path.as_ref().and_then(|path| {
            match start_ipc_server(path, cc.egui_ctx.clone()) {
                Ok(handle) => Some(handle),
                Err(e) => {
                    log::error!("Failed to start IPC server: {}", e);
                    None
                }
            }
        });

        let mut app = Self {
            config,
            panels: HashMap::new(),
            workspaces: vec![Workspace::new("default")],
            active_workspace: 0,
            next_id: 0,
            event_rx,
            event_tx,
            ipc_handle,
            socket_path,
            command_palette_open: false,
            follow_mode: false,
            perf_stats: PerfStats::default(),
            active_dialog: ActiveDialog::None,
        };

        // Create initial terminal
        app.create_terminal(&cc.egui_ctx);

        app
    }

    /// Restore application state from persisted data.
    #[cfg(unix)]
    pub fn from_persisted(
        cc: &eframe::CreationContext<'_>,
        state: PersistedState,
        socket_path: PathBuf,
        config: Config,
    ) -> Result<Self, String> {
        setup_fonts(&cc.egui_ctx);

        let (event_tx, event_rx) = mpsc::channel();

        // Initialize IPC server
        let ipc_handle = match start_ipc_server(&socket_path, cc.egui_ctx.clone()) {
            Ok(handle) => Some(handle),
            Err(e) => {
                log::error!("Failed to start IPC server: {}", e);
                None
            }
        };

        let mut panels = HashMap::new();
        let mut workspaces = Vec::new();

        // Restore all terminals from all workspaces
        for persisted_ws in &state.workspaces {
            let mut ws = Workspace::new(&persisted_ws.name);
            ws.focused_index = persisted_ws.focused_index;

            for persisted_term in &persisted_ws.terminals {
                // Try to restore this terminal
                match unsafe {
                    TerminalPanel::from_persisted(
                        persisted_term.internal_id,
                        persisted_term,
                        &cc.egui_ctx,
                        event_tx.clone(),
                    )
                } {
                    Ok(panel) => {
                        panels.insert(persisted_term.internal_id, panel);
                        ws.panel_order.push(persisted_term.internal_id);

                        // Force redraw by toggling PTY size
                        if let Err(e) = persist::force_redraw(
                            persisted_term.pty_fd,
                            persisted_term.pty_pid,
                        ) {
                            log::warn!(
                                "Failed to force redraw for terminal {}: {}",
                                persisted_term.external_id,
                                e
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to restore terminal {}: {}",
                            persisted_term.external_id,
                            e
                        );
                    }
                }
            }

            // Fix up focused_index if needed
            if ws.focused_index >= ws.panel_order.len() {
                ws.focused_index = ws.panel_order.len().saturating_sub(1);
            }

            workspaces.push(ws);
        }

        // If we failed to restore anything, return an error
        if panels.is_empty() {
            return Err("No terminals could be restored".to_string());
        }

        // Remove any empty workspaces (except keep at least one)
        workspaces.retain(|ws| !ws.panel_order.is_empty());
        if workspaces.is_empty() {
            workspaces.push(Workspace::new("default"));
        }

        let active_workspace = state.active_workspace.min(workspaces.len().saturating_sub(1));

        Ok(Self {
            config,
            panels,
            workspaces,
            active_workspace,
            next_id: state.next_id,
            event_rx,
            event_tx,
            ipc_handle,
            socket_path: Some(socket_path),
            command_palette_open: false,
            follow_mode: false,
            perf_stats: PerfStats::default(),
            active_dialog: ActiveDialog::None,
        })
    }

    /// Convert current state to persisted form.
    #[cfg(unix)]
    pub fn to_persisted_state(&self) -> PersistedState {
        let workspaces = self
            .workspaces
            .iter()
            .map(|ws| {
                let terminals: Vec<PersistedTerminal> = ws
                    .panel_order
                    .iter()
                    .filter_map(|&id| {
                        self.panels.get(&id).map(|panel| panel.to_persisted(id))
                    })
                    .collect();

                PersistedWorkspace {
                    name: ws.name.clone(),
                    panel_order: ws.panel_order.clone(),
                    focused_index: ws.focused_index,
                    terminals,
                }
            })
            .collect();

        PersistedState {
            version: persist::STATE_VERSION,
            workspaces,
            active_workspace: self.active_workspace,
            next_id: self.next_id,
        }
    }

    /// Trigger a restart by saving state and exec'ing a new process.
    #[cfg(unix)]
    pub fn trigger_restart(&self) -> Result<(), String> {
        use std::os::unix::process::CommandExt;

        // 1. Serialize state to temp file
        let state = self.to_persisted_state();
        let state_path = std::path::Path::new("/tmp/manse-restart-state.json");
        state
            .save(state_path)
            .map_err(|e| format!("Failed to save state: {}", e))?;

        // 2. Clear CLOEXEC on all PTY fds
        for panel in self.panels.values() {
            let fd = panel.pty_fd();
            if let Err(e) = persist::clear_cloexec(fd) {
                log::warn!("Failed to clear CLOEXEC on fd {}: {}", fd, e);
            }
        }

        // 3. Build exec args
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe: {}", e))?;

        let socket_path = self
            .socket_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/tmp/manse.sock".to_string());

        // 4. exec (does not return on success)
        let err = std::process::Command::new(&exe)
            .arg("resume")
            .arg("--state-file")
            .arg(state_path)
            .arg("-s")
            .arg(&socket_path)
            .exec();

        // If we get here, exec failed
        Err(format!("exec failed: {}", err))
    }

    /// Log performance stats if enabled and interval has elapsed
    fn maybe_log_perf_stats(&mut self) {
        let interval = self.config.perf_log_interval;
        if interval <= 0.0 {
            return;
        }

        let now = Instant::now();
        let window_start = self.perf_stats.window_start.get_or_insert(now);
        let elapsed = now.duration_since(*window_start);

        if elapsed >= Duration::from_secs_f32(interval) {
            let secs = elapsed.as_secs_f64();
            let fps = self.perf_stats.frame_count as f64 / secs;
            let s = &self.perf_stats;

            // Count unexplained frames (frames not accounted for by known causes)
            let explained = s.pty_events + s.scroll_animation_frames + s.minimized_frames + s.pointer_frames + s.keyboard_frames;
            let mystery = s.frame_count.saturating_sub(explained);

            log::info!(
                "[perf] {:.1}s: frames={} ({:.1} fps) | pty={} scroll={} pointer={} kbd={} focused={} | mystery={}",
                secs,
                s.frame_count,
                fps,
                s.pty_events,
                s.scroll_animation_frames,
                s.pointer_frames,
                s.keyboard_frames,
                s.focused_frames,
                mystery,
            );

            // Reset stats
            self.perf_stats = PerfStats {
                window_start: Some(now),
                ..Default::default()
            };
        }
    }

    fn active_workspace(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    fn active_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// Remove empty workspaces except "default". Adjusts active_workspace index if needed.
    fn cleanup_empty_workspaces(&mut self) {
        let mut i = 0;
        while i < self.workspaces.len() {
            if self.workspaces[i].panel_order.is_empty() && self.workspaces[i].name != "default" {
                self.workspaces.remove(i);
                // Adjust active workspace index if it was after the removed one
                if self.active_workspace > i {
                    self.active_workspace -= 1;
                } else if self.active_workspace == i && self.active_workspace >= self.workspaces.len() {
                    self.active_workspace = self.workspaces.len().saturating_sub(1);
                }
            } else {
                i += 1;
            }
        }
    }

    fn create_terminal(&mut self, ctx: &egui::Context) {
        let id = self.next_id;
        self.next_id += 1;

        // Inherit working directory from focused terminal
        let working_dir = self.focused_panel()
            .and_then(|p| p.current_working_directory.clone());

        let panel = TerminalPanel::new(id, ctx, self.event_tx.clone(), self.socket_path.as_ref(), working_dir);
        self.panels.insert(id, panel);

        // Insert after the currently focused terminal (or at end if empty)
        let ws = self.active_workspace_mut();
        if ws.panel_order.is_empty() {
            ws.panel_order.push(id);
        } else {
            let insert_pos = ws.focused_index + 1;
            ws.panel_order.insert(insert_pos, id);
        }
        ws.invalidate_positions();
    }

    fn focused_panel(&self) -> Option<&TerminalPanel> {
        let ws = self.active_workspace();
        ws.panel_order
            .get(ws.focused_index)
            .and_then(|id| self.panels.get(id))
    }

    fn focused_panel_mut(&mut self) -> Option<&mut TerminalPanel> {
        let focused_id = self.active_workspace().panel_order
            .get(self.active_workspace().focused_index)
            .copied();
        focused_id.and_then(|id| self.panels.get_mut(&id))
    }

    fn focus_next(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index < ws.panel_order.len().saturating_sub(1) {
            ws.focused_index += 1;
        }
    }

    fn focus_prev(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index > 0 {
            ws.focused_index -= 1;
        }
    }

    fn grow_focused(&mut self) {
        if let Some(panel) = self.focused_panel_mut() {
            let current = panel.width_ratio;
            for &ratio in WIDTH_RATIOS.iter() {
                if ratio > current + 0.01 {
                    panel.width_ratio = ratio;
                    break;
                }
            }
        }
        self.active_workspace_mut().invalidate_positions();
    }

    fn shrink_focused(&mut self) {
        if let Some(panel) = self.focused_panel_mut() {
            let current = panel.width_ratio;
            for &ratio in WIDTH_RATIOS.iter().rev() {
                if ratio < current - 0.01 {
                    panel.width_ratio = ratio;
                    break;
                }
            }
        }
        self.active_workspace_mut().invalidate_positions();
    }

    fn swap_with_prev(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index > 0 {
            ws.panel_order.swap(ws.focused_index, ws.focused_index - 1);
            ws.focused_index -= 1;
            ws.invalidate_positions();
        }
    }

    fn swap_with_next(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index < ws.panel_order.len().saturating_sub(1) {
            ws.panel_order.swap(ws.focused_index, ws.focused_index + 1);
            ws.focused_index += 1;
            ws.invalidate_positions();
        }
    }

    fn close_focused(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.panel_order.len() <= 1 {
            return; // Don't close the last terminal
        }

        if let Some(&id) = ws.panel_order.get(ws.focused_index) {
            self.panels.remove(&id);
            let ws = self.active_workspace_mut();
            ws.panel_order.remove(ws.focused_index);

            // Adjust focus index
            if ws.focused_index >= ws.panel_order.len() {
                ws.focused_index = ws.panel_order.len().saturating_sub(1);
            }
            ws.invalidate_positions();
        }
    }

    /// Compute and cache terminal positions for the active workspace.
    /// Returns a reference to the cached positions.
    fn ensure_positions_cached(&mut self, viewport_width: f32) {
        let ws = self.active_workspace();
        // Check if cache is valid
        if (ws.cached_positions.viewport_width - viewport_width).abs() < 0.1
            && ws.cached_positions.positions.len() == ws.panel_order.len()
        {
            return;
        }

        // Recompute positions
        let mut positions = Vec::with_capacity(ws.panel_order.len());
        let mut x_pos = 0.0;
        for &id in &ws.panel_order {
            if let Some(panel) = self.panels.get(&id) {
                let width = panel.pixel_width(viewport_width);
                positions.push((id, x_pos, width));
                x_pos += width;
            }
        }

        let ws = self.active_workspace_mut();
        ws.cached_positions.positions = positions;
        ws.cached_positions.viewport_width = viewport_width;
    }

    fn terminal_x_position(&self, index: usize, viewport_width: f32) -> f32 {
        let ws = self.active_workspace();
        // Use cached positions if available and valid
        if (ws.cached_positions.viewport_width - viewport_width).abs() < 0.1 {
            if let Some(&(_, x, _)) = ws.cached_positions.positions.get(index) {
                return x;
            }
        }
        // Fallback to computing
        let mut x = 0.0;
        for i in 0..index {
            if let Some(&id) = ws.panel_order.get(i) {
                if let Some(panel) = self.panels.get(&id) {
                    x += panel.pixel_width(viewport_width);
                }
            }
        }
        x
    }

    fn total_content_width(&self, viewport_width: f32) -> f32 {
        let ws = self.active_workspace();
        // Use cached positions if available and valid
        if (ws.cached_positions.viewport_width - viewport_width).abs() < 0.1
            && !ws.cached_positions.positions.is_empty()
        {
            // Total width is the last position's x + width
            if let Some(&(_, x, w)) = ws.cached_positions.positions.last() {
                return x + w;
            }
        }
        // Fallback
        ws.panel_order
            .iter()
            .filter_map(|id| self.panels.get(id))
            .map(|p| p.pixel_width(viewport_width))
            .sum()
    }

    fn scroll_to_focused(&mut self, viewport_width: f32) {
        if self.active_workspace().panel_order.is_empty() {
            return;
        }

        let focused_index = self.active_workspace().focused_index;
        let term_x = self.terminal_x_position(focused_index, viewport_width);
        let term_width = self
            .focused_panel()
            .map(|p| p.pixel_width(viewport_width))
            .unwrap_or(0.0);
        let term_right = term_x + term_width;
        let target_offset = self.active_workspace().target_offset;
        let view_right = target_offset + viewport_width;

        let ws = self.active_workspace_mut();
        if term_x < ws.target_offset {
            ws.target_offset = term_x;
        } else if term_right > view_right {
            ws.target_offset = term_right - viewport_width;
        }

        let max_scroll = (self.total_content_width(viewport_width) - viewport_width).max(0.0);
        let ws = self.active_workspace_mut();
        ws.target_offset = ws.target_offset.clamp(0.0, max_scroll);
    }

    fn update_scroll(&mut self) {
        let ws = self.active_workspace_mut();
        let diff = ws.target_offset - ws.scroll_offset;
        if diff.abs() > 0.5 {
            ws.scroll_offset += diff * SCROLL_EASING;
        } else {
            ws.scroll_offset = ws.target_offset;
        }
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok((id, event)) = self.event_rx.try_recv() {
            self.perf_stats.pty_events += 1;
            match event {
                PtyEvent::Exit => {
                    // Remove the exited terminal from its workspace
                    for ws in &mut self.workspaces {
                        if let Some(pos) = ws.panel_order.iter().position(|&x| x == id) {
                            ws.panel_order.remove(pos);

                            // Adjust focus within this workspace
                            if ws.focused_index >= ws.panel_order.len() {
                                ws.focused_index = ws.panel_order.len().saturating_sub(1);
                            }
                            ws.invalidate_positions();
                            break;
                        }
                    }

                    self.panels.remove(&id);
                    self.cleanup_empty_workspaces();

                    // Check if all terminals are closed
                    let total_terminals: usize = self.workspaces.iter()
                        .map(|ws| ws.panel_order.len())
                        .sum();
                    if total_terminals == 0 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                }
                PtyEvent::Title(title) => {
                    if let Some(panel) = self.panels.get_mut(&id) {
                        panel.title = title;
                    }
                }
                PtyEvent::WorkingDirectory(path) => {
                    if let Some(panel) = self.panels.get_mut(&id) {
                        panel.current_working_directory = Some(PathBuf::from(path));
                    }
                }
                _ => {}
            }
        }
    }

    fn execute_command(&mut self, cmd: Command, ctx: &egui::Context) {
        match cmd {
            Command::NewTerminal => {
                let ws = self.active_workspace_mut();
                let new_index = ws.focused_index + 1;
                self.create_terminal(ctx);
                self.active_workspace_mut().focused_index = new_index;
            }
            Command::CloseTerminal => {
                // Show confirmation dialog instead of closing immediately
                self.active_dialog = ActiveDialog::ConfirmClose;
            }
            Command::FocusPrevious => self.focus_prev(),
            Command::FocusNext => self.focus_next(),
            Command::SwapWithPrevious => self.swap_with_prev(),
            Command::SwapWithNext => self.swap_with_next(),
            Command::ShrinkTerminal => self.shrink_focused(),
            Command::GrowTerminal => self.grow_focused(),
            Command::FollowMode => self.follow_mode = true,
            Command::SetDescription => {
                // Get current description as starting value
                let current = self.focused_panel()
                    .map(|p| p.description.clone())
                    .unwrap_or_default();
                self.active_dialog = ActiveDialog::SetDescription { input: current };
            }
        }
    }

    /// Build a mapping of letter index (0-25) to (workspace_idx, terminal_idx)
    fn build_follow_targets(&self) -> Vec<(usize, usize)> {
        let mut targets = Vec::new();
        for (ws_idx, ws) in self.workspaces.iter().enumerate() {
            for (term_idx, _) in ws.panel_order.iter().enumerate() {
                if targets.len() >= 26 {
                    break;
                }
                targets.push((ws_idx, term_idx));
            }
            if targets.len() >= 26 {
                break;
            }
        }
        targets
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Don't process shortcuts when a dialog is active (dialog handles its own input)
        if !matches!(self.active_dialog, ActiveDialog::None) {
            return;
        }

        // Escape closes command palette or follow mode
        if self.command_palette_open {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.command_palette_open = false;
                return;
            }
        }

        if self.follow_mode {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.follow_mode = false;
                return;
            }

            // Check for letter keys a-z
            let letter_keys = [
                egui::Key::A, egui::Key::B, egui::Key::C, egui::Key::D, egui::Key::E,
                egui::Key::F, egui::Key::G, egui::Key::H, egui::Key::I, egui::Key::J,
                egui::Key::K, egui::Key::L, egui::Key::M, egui::Key::N, egui::Key::O,
                egui::Key::P, egui::Key::Q, egui::Key::R, egui::Key::S, egui::Key::T,
                egui::Key::U, egui::Key::V, egui::Key::W, egui::Key::X, egui::Key::Y,
                egui::Key::Z,
            ];

            for (idx, &key) in letter_keys.iter().enumerate() {
                if ctx.input(|i| i.key_pressed(key)) {
                    let targets = self.build_follow_targets();
                    if let Some(&(ws_idx, term_idx)) = targets.get(idx) {
                        self.active_workspace = ws_idx;
                        self.workspaces[ws_idx].focused_index = term_idx;
                    }
                    self.follow_mode = false;
                    return;
                }
            }

            // Don't process other input while in follow mode
            return;
        }

        let modifiers = ctx.input(|i| i.modifiers);

        // Cmd+P: Toggle command palette (check before other shortcuts)
        if modifiers.command && ctx.input(|i| i.key_pressed(egui::Key::P)) {
            self.command_palette_open = !self.command_palette_open;
            return;
        }

        // Don't process other shortcuts when command palette is open
        if self.command_palette_open {
            return;
        }

        // Only process shortcuts when Cmd is held (macOS-style keybindings)
        if !modifiers.command {
            return;
        }

        // Use input_mut with consume_key to prevent default handlers (like zoom) from processing
        ctx.input_mut(|i| {
            // Cmd+T: New terminal (matches WezTerm)
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::T) {
                self.execute_command(Command::NewTerminal, ctx);
            }

            // Cmd+W: Close focused terminal
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::W) {
                self.execute_command(Command::CloseTerminal, ctx);
            }

            // Cmd+Shift+[ (reported as Cmd+{): Swap with previous
            // On macOS, Shift+[ produces OpenCurlyBracket, not OpenBracket with shift
            if i.key_pressed(egui::Key::OpenCurlyBracket) && i.modifiers.command {
                self.execute_command(Command::SwapWithPrevious, ctx);
            }
            // Cmd+[: Focus previous
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::OpenBracket) {
                self.execute_command(Command::FocusPrevious, ctx);
            }

            // Cmd+Shift+] (reported as Cmd+}): Swap with next
            if i.key_pressed(egui::Key::CloseCurlyBracket) && i.modifiers.command {
                self.execute_command(Command::SwapWithNext, ctx);
            }
            // Cmd+]: Focus next
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::CloseBracket) {
                self.execute_command(Command::FocusNext, ctx);
            }

            // Cmd+-: Shrink focused (consume to prevent default zoom behavior)
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus) {
                self.execute_command(Command::ShrinkTerminal, ctx);
            }

            // Cmd+=: Grow focused (consume to prevent default zoom behavior)
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals) {
                self.execute_command(Command::GrowTerminal, ctx);
            }

            // Cmd+J: Follow mode (jump to terminal by letter)
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::J) {
                self.execute_command(Command::FollowMode, ctx);
            }

            // Cmd+D: Set terminal description
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::D) {
                self.execute_command(Command::SetDescription, ctx);
            }
        });
    }

    fn process_ipc(&mut self, ctx: &egui::Context) {
        let Some(handle) = &self.ipc_handle else {
            return;
        };

        for pending in handle.poll() {
            self.perf_stats.ipc_requests += 1;
            let response = match pending.request {
                Request::Ping => Response::ok(),
                Request::Restart => {
                    // Respond OK first, then trigger restart
                    pending.respond(Response::ok());

                    #[cfg(unix)]
                    if let Err(e) = self.trigger_restart() {
                        log::error!("Restart failed: {}", e);
                        // If restart failed, close the window so we don't leave a broken state
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    // If exec succeeded, we won't reach here
                    // If exec failed, we already responded and logged error
                    continue;
                }
                Request::TermRename { ref terminal, ref title } => {
                    let panel = self.panels.values_mut()
                        .find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        panel.custom_title = Some(title.clone());
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermDesc { ref terminal, ref description } => {
                    let panel = self.panels.values_mut()
                        .find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        panel.description = description.clone();
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermEmoji { ref terminal, ref emoji } => {
                    let panel = self.panels.values_mut()
                        .find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        if emoji.is_empty() {
                            panel.emoji = None;
                        } else {
                            panel.emoji = Some(emoji.clone());
                        }
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermNotify { ref terminal } => {
                    let panel = self.panels.values_mut()
                        .find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        panel.notified = true;
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermToWorkspace { ref terminal, ref workspace_name } => {
                    // Find the panel's internal id
                    let panel_id = self.panels.iter()
                        .find(|(_, p)| p.id == *terminal)
                        .map(|(&id, _)| id);

                    match panel_id {
                        Some(id) => {
                            // Check if terminal is already in target workspace
                            let current_ws_idx = self.workspaces.iter()
                                .position(|ws| ws.panel_order.contains(&id));

                            if let Some(ws_idx) = current_ws_idx {
                                if self.workspaces[ws_idx].name == *workspace_name {
                                    // Already in target workspace, just switch to it
                                    self.active_workspace = ws_idx;
                                    pending.respond(Response::ok());
                                    continue;
                                }
                            }

                            // Remove from current workspace
                            for ws in &mut self.workspaces {
                                if let Some(pos) = ws.panel_order.iter().position(|&x| x == id) {
                                    ws.panel_order.remove(pos);
                                    // Adjust focused_index if needed
                                    if ws.focused_index >= ws.panel_order.len() && !ws.panel_order.is_empty() {
                                        ws.focused_index = ws.panel_order.len() - 1;
                                    }
                                    ws.invalidate_positions();
                                    break;
                                }
                            }

                            // Find or create target workspace
                            let target_ws_idx = self.workspaces.iter()
                                .position(|ws| ws.name == *workspace_name);

                            let target_ws_idx = match target_ws_idx {
                                Some(idx) => idx,
                                None => {
                                    // Create new workspace
                                    self.workspaces.push(Workspace::new(workspace_name));
                                    self.workspaces.len() - 1
                                }
                            };

                            // Add terminal to target workspace
                            self.workspaces[target_ws_idx].panel_order.push(id);
                            self.workspaces[target_ws_idx].focused_index =
                                self.workspaces[target_ws_idx].panel_order.len() - 1;
                            self.workspaces[target_ws_idx].invalidate_positions();

                            // Switch to the target workspace
                            self.active_workspace = target_ws_idx;

                            // Clean up empty workspaces
                            self.cleanup_empty_workspaces();

                            Response::ok()
                        }
                        None => Response::error(format!("Terminal not found: {}", terminal)),
                    }
                }
            };
            pending.respond(response);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Track frame for perf stats
        self.perf_stats.frame_count += 1;

        // Track input activity for perf debugging
        ctx.input(|i| {
            if i.focused {
                self.perf_stats.focused_frames += 1;
            }
            if i.pointer.is_moving() || i.pointer.any_down() || i.pointer.any_released() {
                self.perf_stats.pointer_frames += 1;
            }
            if i.keys_down.len() > 0 || i.events.iter().any(|e| matches!(e, egui::Event::Key { .. } | egui::Event::Text(_))) {
                self.perf_stats.keyboard_frames += 1;
            }
        });

        // Skip rendering when minimized (window definitely not visible)
        let is_minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if is_minimized {
            self.perf_stats.minimized_frames += 1;
            // Still process events so terminals don't buffer forever
            self.process_events(ctx);
            self.process_ipc(ctx);
            // Use slow refresh rate when minimized to save battery
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
            self.maybe_log_perf_stats();
            return;
        }

        // Request repaint during scroll animation
        let ws = self.active_workspace();
        let is_scrolling = (ws.scroll_offset - ws.target_offset).abs() > 0.5;
        if is_scrolling {
            self.perf_stats.scroll_animation_frames += 1;
            ctx.request_repaint();
        }

        // Process PTY events
        self.process_events(ctx);

        // Process IPC commands (background thread triggers repaint when requests arrive)
        self.process_ipc(ctx);

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Clear notification on focused terminal
        if let Some(panel) = self.focused_panel_mut() {
            panel.notified = false;
        }

        // Update scroll animation
        self.update_scroll();

        // Sidebar (left)
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(self.config.sidebar.width)
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                if let Some(action) =
                    sidebar::render(ui, &self.workspaces, self.active_workspace, &self.panels, self.follow_mode, &self.config.sidebar)
                {
                    match action {
                        sidebar::SidebarAction::SwitchWorkspace(ws_idx) => {
                            self.active_workspace = ws_idx;
                        }
                        sidebar::SidebarAction::FocusTerminal { workspace, terminal } => {
                            self.active_workspace = workspace;
                            self.workspaces[workspace].focused_index = terminal;
                        }
                    }
                }
            });

        // Main terminal area
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                let total_width = ui.available_width();

                // Status bar at top with terminal indicators
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(20, 20, 20))
                    .show(ui, |ui| {
                        ui.set_min_width(total_width);
                        ui.set_height(28.0);
                        ui.horizontal_centered(|ui| {
                            status_bar::render(
                                ui,
                                self.active_workspace(),
                                self.focused_panel(),
                            );
                        });
                    });

                // Terminal area with padding
                let padding = 4.0;
                let available = ui.available_size();
                let padded_height = available.y - padding * 2.0;
                let viewport_width = available.x - padding * 2.0;

                // Scroll to focused terminal
                self.scroll_to_focused(viewport_width);

                // Ensure terminal positions are cached
                self.ensure_positions_cached(viewport_width);

                // Get active workspace state (no clone needed - use cached positions)
                let scroll_offset = self.active_workspace().scroll_offset;
                let focused_index = self.active_workspace().focused_index;
                // Clone the cached positions (small, fixed-size tuples - much cheaper than panel_order clone + recompute)
                let terminal_positions = self.active_workspace().cached_positions.positions.clone();

                // Add top padding
                ui.add_space(padding);

                let border_width = 2.0;
                let terminal_font_size = self.config.terminal_font_size;

                // Determine visible range (with some margin for partial visibility)
                let view_left = scroll_offset;
                let view_right = scroll_offset + viewport_width;

                // Render terminals using absolute positioning within a fixed area
                let terminal_area = ui.available_rect_before_wrap();
                let base_x = terminal_area.left() + padding;
                let base_y = terminal_area.top();

                for (idx, &(id, term_x, term_width)) in terminal_positions.iter().enumerate() {
                    let term_right = term_x + term_width;

                    // Skip terminals that are completely outside the viewport
                    if term_right < view_left || term_x > view_right {
                        continue;
                    }

                    if let Some(panel) = self.panels.get_mut(&id) {
                        let is_focused = idx == focused_index;

                        // Calculate screen position
                        let screen_x = base_x + term_x - scroll_offset;
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(screen_x, base_y),
                            egui::vec2(term_width, padded_height),
                        );

                        // Allocate the rect and create a child UI
                        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));

                        // Create a frame for the terminal
                        let frame = if is_focused {
                            egui::Frame::NONE
                                .stroke(egui::Stroke::new(border_width, egui::Color32::from_rgb(100, 150, 255)))
                        } else {
                            egui::Frame::NONE
                        };

                        // Account for border in inner size
                        let inner_width = term_width - border_width * 2.0;
                        let inner_height = padded_height - border_width * 2.0;

                        // Check if any dialog is open (terminals shouldn't steal focus)
                        let dialog_open = !matches!(self.active_dialog, ActiveDialog::None);

                        frame.show(&mut child_ui, |ui| {
                            // Render terminal with slightly smaller size to fit border
                            let font = TerminalFont::new(FontSettings {
                                font_type: egui::FontId::monospace(terminal_font_size),
                            });
                            let term_view = TerminalView::new(ui, &mut panel.backend)
                                .set_focus(is_focused && !dialog_open)
                                .set_font(font)
                                .set_size(egui::vec2(inner_width, inner_height));
                            let response = ui.add(term_view);

                            // Only request focus if no dialog is open
                            if is_focused && !dialog_open {
                                response.request_focus();
                            }
                        });
                    }
                }

                // Reserve the space for the terminal area
                ui.allocate_space(egui::vec2(viewport_width + padding * 2.0, padded_height));
            });

        // Command palette overlay
        if self.command_palette_open {
            let result = command_palette::render(ctx);

            if result.background_clicked {
                self.command_palette_open = false;
            }

            if let Some(cmd) = result.selected_command {
                self.command_palette_open = false;
                self.execute_command(cmd, ctx);
            }
        }

        // Dialog overlays
        self.render_dialogs(ctx);

        // Log perf stats periodically if enabled
        self.maybe_log_perf_stats();
    }
}

impl App {
    fn render_dialogs(&mut self, ctx: &egui::Context) {
        match &self.active_dialog {
            ActiveDialog::None => {}
            ActiveDialog::ConfirmClose => {
                self.render_confirm_close_dialog(ctx);
            }
            ActiveDialog::SetDescription { .. } => {
                self.render_set_description_dialog(ctx);
            }
        }
    }

    fn render_confirm_close_dialog(&mut self, ctx: &egui::Context) {
        #[allow(deprecated)]
        let screen_rect = ctx.screen_rect();

        // Semi-transparent background
        egui::Area::new(egui::Id::new("dialog_bg"))
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_black_alpha(128),
                );
                if response.clicked() {
                    self.active_dialog = ActiveDialog::None;
                }
            });

        // Dialog window
        let dialog_width = 300.0;
        let dialog_x = (screen_rect.width() - dialog_width) / 2.0;
        let dialog_y = screen_rect.height() * 0.3;

        let mut should_close = false;
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
                                egui::RichText::new("Close")
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(180, 60, 60));

                            if ui.add(close_btn).clicked() {
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

        // Handle enter key for confirm
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            should_confirm = true;
        }

        if should_close {
            self.active_dialog = ActiveDialog::None;
        } else if should_confirm {
            self.active_dialog = ActiveDialog::None;
            self.close_focused();
        }
    }

    fn render_set_description_dialog(&mut self, ctx: &egui::Context) {
        #[allow(deprecated)]
        let screen_rect = ctx.screen_rect();

        // Semi-transparent background
        egui::Area::new(egui::Id::new("dialog_bg_desc"))
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_black_alpha(128),
                );
                if response.clicked() {
                    self.active_dialog = ActiveDialog::None;
                }
            });

        // Dialog window
        let dialog_width = 400.0;
        let dialog_x = (screen_rect.width() - dialog_width) / 2.0;
        let dialog_y = screen_rect.height() * 0.3;

        let mut should_close = false;
        let mut should_confirm = false;
        let mut new_input = None;

        // Extract current input value
        let current_input = match &self.active_dialog {
            ActiveDialog::SetDescription { input } => input.clone(),
            _ => String::new(),
        };

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
                            let mut input = current_input.clone();
                            let text_edit = egui::TextEdit::singleline(&mut input)
                                .desired_width(dialog_width - 40.0)
                                .hint_text("Enter description...");
                            let response = ui.add(text_edit);

                            // Always request focus for the text input
                            response.request_focus();

                            if input != current_input {
                                new_input = Some(input.clone());
                            }

                            // Enter to confirm (check globally since we have focus)
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                should_confirm = true;
                                new_input = Some(input);
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
                                egui::RichText::new("Save")
                                    .color(egui::Color32::WHITE),
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

        // Update input if changed
        if let Some(input) = new_input {
            if !should_confirm && !should_close {
                self.active_dialog = ActiveDialog::SetDescription { input };
            }
        }

        if should_close {
            self.active_dialog = ActiveDialog::None;
        } else if should_confirm {
            // Get the final input value
            let description = match &self.active_dialog {
                ActiveDialog::SetDescription { input } => input.clone(),
                _ => String::new(),
            };

            // Update the focused panel's description
            if let Some(panel) = self.focused_panel_mut() {
                panel.description = description;
            }

            self.active_dialog = ActiveDialog::None;
        }
    }
}

