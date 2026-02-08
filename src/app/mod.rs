mod input;
mod ipc;
mod perf;
mod terminals;

use crate::config::Config;
use egui_term::TerminalTheme;
use crate::fonts;
use crate::ipc_protocol::{start_ipc_server, IpcHandle};
use crate::persist::{self, PersistedState, PersistedTerminal, PersistedWorkspace};
use crate::terminal::TerminalPanel;
use crate::ui::{
    command_palette, dialogs_state, sidebar, status_bar, terminal_strip, ActiveDialog, DialogAction,
};
use crate::util::layout;
use crate::workspace::Workspace;
use eframe::egui;
use egui_term::PtyEvent;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use self::perf::PerfStats;

/// Width ratios for terminal panels
pub const WIDTH_RATIOS: [f32; 4] = [0.333, 0.5, 0.667, 1.0];

/// The scrolling window manager
pub struct App {
    /// Application configuration
    config: Config,
    /// Terminal color theme (cached from config)
    terminal_theme: TerminalTheme,
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
    /// Whether move-to-spot mode is active (move terminal to position by letter)
    move_to_spot_mode: bool,
    /// Whether the sidebar is visible
    sidebar_visible: bool,
    /// Performance tracking stats
    perf_stats: PerfStats,
    /// Active dialog (confirmation, input, etc.)
    active_dialog: ActiveDialog,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, socket_path: Option<PathBuf>, config: Config) -> Self {
        // Configure fonts with emoji support
        fonts::setup_fonts(&cc.egui_ctx, config.font_family.as_deref());

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

        let terminal_theme = config.build_theme();

        let mut app = Self {
            config,
            terminal_theme,
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
            move_to_spot_mode: false,
            sidebar_visible: true,
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
        fonts::setup_fonts(&cc.egui_ctx, config.font_family.as_deref());

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
        let terminal_theme = config.build_theme();

        Ok(Self {
            config,
            terminal_theme,
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
            move_to_spot_mode: false,
            sidebar_visible: true,
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.perf_stats.on_frame(ctx);

        // Skip rendering when minimized (window definitely not visible)
        let is_minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if is_minimized {
            self.perf_stats.on_minimized();
            // Still process events so terminals don't buffer forever
            self.process_events(ctx);
            self.process_ipc(ctx);
            // Use slow refresh rate when minimized to save battery
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
            self.perf_stats.maybe_log(self.config.perf_log_interval);
            return;
        }

        // Request repaint during scroll animation
        let ws = self.active_workspace();
        let is_scrolling = layout::is_animating(ws.scroll_offset, ws.target_offset);
        if is_scrolling {
            self.perf_stats.on_scroll_anim();
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
        if self.sidebar_visible {
            egui::SidePanel::left("sidebar")
                .resizable(false)
                .exact_width(self.config.sidebar.width)
                .frame(egui::Frame::NONE.fill(self.config.ui_colors.sidebar_background))
                .show(ctx, |ui| {
                    if let Some(action) =
                        sidebar::render(ui, &self.workspaces, self.active_workspace, &self.panels, self.follow_mode || self.move_to_spot_mode, &self.config.sidebar, &self.config.icons, &self.config.ui_colors)
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
        }

        // Main terminal area
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                let total_width = ui.available_width();

                // Calculate viewport dimensions early so we can cache positions
                // before rendering the status bar minimap
                let padding = 4.0;
                let available = ui.available_size();
                // Reserve 28px for status bar height
                let padded_height = available.y - padding * 2.0 - 28.0;
                let viewport_width = available.x - padding * 2.0;

                // Ensure terminal positions are cached before status bar render
                self.ensure_positions_cached(viewport_width);

                // Scroll to focused terminal
                self.scroll_to_focused(viewport_width);

                // Build minimap state from cached positions
                let minimap_state = {
                    let ws = self.active_workspace();
                    let positions: Vec<(f32, f32)> = ws
                        .cached_positions
                        .positions
                        .iter()
                        .map(|(_, x, w)| (*x, *w))
                        .collect();

                    if !positions.is_empty() {
                        Some(status_bar::MinimapState {
                            positions,
                            scroll_offset: ws.scroll_offset,
                            viewport_width,
                        })
                    } else {
                        None
                    }
                };

                egui::Frame::NONE
                    .fill(self.config.ui_colors.status_bar_background)
                    .show(ui, |ui| {
                        ui.set_min_width(total_width);
                        ui.set_height(28.0);
                        ui.horizontal_centered(|ui| {
                            status_bar::render(
                                ui,
                                self.active_workspace(),
                                self.focused_panel(),
                                minimap_state.as_ref(),
                                &self.config.status_bar,
                                &self.config.ui_colors,
                            );
                        });
                    });

                let dialog_open = !matches!(self.active_dialog, ActiveDialog::None);
                let terminal_state = terminal_strip::TerminalStripState {
                    scroll_offset: self.active_workspace().scroll_offset,
                    focused_index: self.active_workspace().focused_index,
                    positions: self.active_workspace().cached_positions.positions.clone(),
                };

                if let Some(clicked_idx) = terminal_strip::render(
                    ui,
                    &self.config,
                    &self.terminal_theme,
                    &terminal_state,
                    &mut self.panels,
                    dialog_open,
                    viewport_width,
                    padded_height,
                    padding,
                ) {
                    self.workspaces[self.active_workspace].focused_index = clicked_idx;
                }
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

        let dialog_action = dialogs_state::render_dialogs(ctx, &mut self.active_dialog);
        match dialog_action {
            DialogAction::None => {}
            DialogAction::ConfirmClose => self.close_focused(),
            DialogAction::SaveDescription(description) => {
                if let Some(panel) = self.focused_panel_mut() {
                    panel.description = description;
                }
            }
        }

        self.perf_stats.maybe_log(self.config.perf_log_interval);
    }
}
