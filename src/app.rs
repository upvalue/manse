use crate::command::Command;
use crate::ipc::{IpcServer, Request, Response};
use crate::terminal::TerminalPanel;
use crate::ui::{command_palette, sidebar, status_bar};
use crate::workspace::Workspace;
use eframe::egui;
use egui_term::{PtyEvent, TerminalView};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use uuid::Uuid;

/// Width ratios for terminal panels
pub const WIDTH_RATIOS: [f32; 4] = [0.333, 0.5, 0.667, 1.0];

/// Scroll animation easing factor
const SCROLL_EASING: f32 = 0.15;

/// The scrolling window manager
pub struct App {
    /// Terminal panels (global pool)
    panels: BTreeMap<u64, TerminalPanel>,
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
    /// IPC server for external control
    ipc_server: Option<IpcServer>,
    /// Socket path for IPC (passed to terminal env)
    socket_path: Option<PathBuf>,
    /// Whether the command palette is open
    command_palette_open: bool,
    /// Whether follow mode is active (jump to terminal by letter)
    follow_mode: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, socket_path: Option<PathBuf>) -> Self {
        let (event_tx, event_rx) = mpsc::channel();

        // Initialize IPC server if socket path provided
        let ipc_server = socket_path.as_ref().and_then(|path| {
            match IpcServer::new(path) {
                Ok(server) => {
                    log::info!("IPC server listening on: {}", path.display());
                    Some(server)
                }
                Err(e) => {
                    log::error!("Failed to start IPC server: {}", e);
                    None
                }
            }
        });

        let mut app = Self {
            panels: BTreeMap::new(),
            workspaces: vec![Workspace::new("default")],
            active_workspace: 0,
            next_id: 0,
            event_rx,
            event_tx,
            ipc_server,
            socket_path,
            command_palette_open: false,
            follow_mode: false,
        };

        // Create initial terminal
        app.create_terminal(&cc.egui_ctx);

        app
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

        let panel = TerminalPanel::new(id, ctx, self.event_tx.clone(), self.socket_path.as_ref());
        self.panels.insert(id, panel);
        self.active_workspace_mut().panel_order.push(id);
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
        }
    }

    fn terminal_x_position(&self, index: usize, viewport_width: f32) -> f32 {
        let ws = self.active_workspace();
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
                _ => {}
            }
        }
    }

    fn execute_command(&mut self, cmd: Command, ctx: &egui::Context) {
        match cmd {
            Command::NewTerminal => {
                self.create_terminal(ctx);
                let ws = self.active_workspace_mut();
                ws.focused_index = ws.panel_order.len() - 1;
            }
            Command::CloseTerminal => self.close_focused(),
            Command::FocusPrevious => self.focus_prev(),
            Command::FocusNext => self.focus_next(),
            Command::ShrinkTerminal => self.shrink_focused(),
            Command::GrowTerminal => self.grow_focused(),
            Command::FollowMode => self.follow_mode = true,
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

        // Only process shortcuts when Ctrl (or Cmd on Mac) is held
        if !modifiers.ctrl && !modifiers.command {
            return;
        }

        ctx.input(|i| {
            // Ctrl+N: New terminal
            if i.key_pressed(egui::Key::N) {
                self.execute_command(Command::NewTerminal, ctx);
            }

            // Ctrl+W: Close focused terminal
            if i.key_pressed(egui::Key::W) {
                self.execute_command(Command::CloseTerminal, ctx);
            }

            // Ctrl+H: Focus previous
            if i.key_pressed(egui::Key::H) {
                self.execute_command(Command::FocusPrevious, ctx);
            }

            // Ctrl+L: Focus next
            if i.key_pressed(egui::Key::L) {
                self.execute_command(Command::FocusNext, ctx);
            }

            // Ctrl+,: Shrink focused
            if i.key_pressed(egui::Key::Comma) {
                self.execute_command(Command::ShrinkTerminal, ctx);
            }

            // Ctrl+.: Grow focused
            if i.key_pressed(egui::Key::Period) {
                self.execute_command(Command::GrowTerminal, ctx);
            }
        });
    }

    fn process_ipc(&mut self, _ctx: &egui::Context) {
        let Some(server) = &mut self.ipc_server else {
            return;
        };

        let requests = server.poll();

        // Collect responses first, then send them
        let responses: Vec<(usize, Response)> = requests
            .into_iter()
            .map(|(client_idx, request)| {
                let response = match request {
                    Request::Ping => Response::ok(),
                    Request::TermRename { terminal, title } => {
                        // Parse the UUID and find the terminal
                        match Uuid::parse_str(&terminal) {
                            Ok(target_uuid) => {
                                // Find the panel with matching UUID
                                let panel = self.panels.values_mut()
                                    .find(|p| p.uuid == target_uuid);

                                if let Some(panel) = panel {
                                    panel.custom_title = Some(title);
                                    Response::ok()
                                } else {
                                    Response::error(format!("Terminal not found: {}", terminal))
                                }
                            }
                            Err(_) => Response::error(format!("Invalid UUID: {}", terminal)),
                        }
                    }
                    Request::TermDesc { terminal, description } => {
                        match Uuid::parse_str(&terminal) {
                            Ok(target_uuid) => {
                                let panel = self.panels.values_mut()
                                    .find(|p| p.uuid == target_uuid);

                                if let Some(panel) = panel {
                                    panel.description = description;
                                    Response::ok()
                                } else {
                                    Response::error(format!("Terminal not found: {}", terminal))
                                }
                            }
                            Err(_) => Response::error(format!("Invalid UUID: {}", terminal)),
                        }
                    }
                    Request::TermToWorkspace { terminal, workspace_name } => {
                        match Uuid::parse_str(&terminal) {
                            Ok(target_uuid) => {
                                // Find the panel's internal id
                                let panel_id = self.panels.iter()
                                    .find(|(_, p)| p.uuid == target_uuid)
                                    .map(|(&id, _)| id);

                                match panel_id {
                                    Some(id) => {
                                        // Check if terminal is already in target workspace
                                        let current_ws_idx = self.workspaces.iter()
                                            .position(|ws| ws.panel_order.contains(&id));

                                        if let Some(ws_idx) = current_ws_idx {
                                            if self.workspaces[ws_idx].name == workspace_name {
                                                // Already in target workspace, just switch to it
                                                self.active_workspace = ws_idx;
                                                return (client_idx, Response::ok());
                                            }
                                        }

                                        // Remove from current workspace
                                        for ws in &mut self.workspaces {
                                            if let Some(pos) = ws.panel_order.iter().position(|&x| x == id) {
                                                ws.panel_order.remove(pos);
                                                // Adjust focused_index if needed
                                                if ws.focused_index >= ws.panel_order.len() && ws.panel_order.len() > 0 {
                                                    ws.focused_index = ws.panel_order.len() - 1;
                                                }
                                                break;
                                            }
                                        }

                                        // Find or create target workspace
                                        let target_ws_idx = self.workspaces.iter()
                                            .position(|ws| ws.name == workspace_name);

                                        let target_ws_idx = match target_ws_idx {
                                            Some(idx) => idx,
                                            None => {
                                                // Create new workspace
                                                self.workspaces.push(Workspace::new(&workspace_name));
                                                self.workspaces.len() - 1
                                            }
                                        };

                                        // Add terminal to target workspace
                                        self.workspaces[target_ws_idx].panel_order.push(id);
                                        self.workspaces[target_ws_idx].focused_index =
                                            self.workspaces[target_ws_idx].panel_order.len() - 1;

                                        // Switch to the target workspace
                                        self.active_workspace = target_ws_idx;

                                        // Clean up empty workspaces
                                        self.cleanup_empty_workspaces();

                                        Response::ok()
                                    }
                                    None => Response::error(format!("Terminal not found: {}", terminal)),
                                }
                            }
                            Err(_) => Response::error(format!("Invalid UUID: {}", terminal)),
                        }
                    }
                };
                (client_idx, response)
            })
            .collect();

        // Now send responses
        let server = self.ipc_server.as_mut().unwrap();
        for (client_idx, response) in responses {
            server.respond(client_idx, &response);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Always request repaint - terminals need continuous updates for:
        // - Cursor blinking
        // - Async command output
        // - PTY activity
        ctx.request_repaint();

        // Process PTY events
        self.process_events(ctx);

        // Process IPC commands
        self.process_ipc(ctx);

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Update scroll animation
        self.update_scroll();

        // Sidebar (left)
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(300.0)
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                if let Some(action) =
                    sidebar::render(ui, &self.workspaces, self.active_workspace, &self.panels, self.follow_mode)
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

                // Get active workspace state
                let scroll_offset = self.active_workspace().scroll_offset;
                let panel_order = self.active_workspace().panel_order.clone();
                let focused_index = self.active_workspace().focused_index;

                // Add top padding
                ui.add_space(padding);

                // Calculate terminal positions and visibility
                let border_width = 2.0;
                let mut terminal_positions: Vec<(u64, f32, f32)> = Vec::new(); // (id, x_start, width)
                let mut x_pos = 0.0;
                for &id in &panel_order {
                    if let Some(panel) = self.panels.get(&id) {
                        let width = panel.pixel_width(viewport_width);
                        terminal_positions.push((id, x_pos, width));
                        x_pos += width;
                    }
                }

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

                        frame.show(&mut child_ui, |ui| {
                            // Render terminal with slightly smaller size to fit border
                            let term_view = TerminalView::new(ui, &mut panel.backend)
                                .set_focus(is_focused)
                                .set_size(egui::vec2(inner_width, inner_height));
                            let response = ui.add(term_view);

                            // Always keep the focused terminal with keyboard focus
                            if is_focused {
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
    }
}

