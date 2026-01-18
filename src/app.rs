use crate::ipc::{IpcServer, Request, Response};
use eframe::egui;
use egui_term::{BackendSettings, PtyEvent, TerminalBackend, TerminalView};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use uuid::Uuid;

/// Width ratios for terminal panels
const WIDTH_RATIOS: [f32; 4] = [0.333, 0.5, 0.667, 1.0];

/// Scroll animation easing factor
const SCROLL_EASING: f32 = 0.15;

/// A workspace containing a horizontal strip of terminals
struct Workspace {
    /// Unique identifier
    uuid: Uuid,
    /// Workspace name
    name: String,
    /// Order of panels in this workspace (left to right)
    panel_order: Vec<u64>,
    /// Currently focused panel index within this workspace
    focused_index: usize,
    /// Current scroll offset (animated)
    scroll_offset: f32,
    /// Target scroll offset
    target_offset: f32,
}

impl Workspace {
    fn new(name: impl Into<String>) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            name: name.into(),
            panel_order: Vec::new(),
            focused_index: 0,
            scroll_offset: 0.0,
            target_offset: 0.0,
        }
    }
}

/// A terminal panel in the window manager
struct TerminalPanel {
    /// Unique identifier for external reference
    uuid: Uuid,
    /// Internal ID for PTY event routing
    id: u64,
    backend: TerminalBackend,
    width_ratio: f32,
    /// Terminal title (from shell escape sequences)
    title: String,
    /// Custom title set via IPC (overrides natural title when Some)
    custom_title: Option<String>,
}

impl TerminalPanel {
    fn new(
        id: u64,
        ctx: &egui::Context,
        event_tx: Sender<(u64, PtyEvent)>,
        socket_path: Option<&PathBuf>,
    ) -> Self {
        let uuid = Uuid::new_v4();

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        // Set environment variables for the terminal
        let mut env = HashMap::new();
        env.insert("MANSE_TERMINAL".to_string(), uuid.to_string());
        if let Some(path) = socket_path {
            env.insert("MANSE_SOCKET".to_string(), path.display().to_string());
        }

        let settings = BackendSettings {
            shell,
            working_directory: std::env::current_dir().ok(),
            env,
            ..Default::default()
        };

        let backend = TerminalBackend::new(id, ctx.clone(), event_tx, settings)
            .expect("Failed to create terminal backend");

        Self {
            uuid,
            id,
            backend,
            width_ratio: 1.0,
            title: String::from("Terminal"),
            custom_title: None,
        }
    }

    /// Returns the display title (custom title if set, otherwise natural title)
    fn display_title(&self) -> &str {
        self.custom_title.as_deref().unwrap_or(&self.title)
    }

    fn pixel_width(&self, viewport_width: f32) -> f32 {
        viewport_width * self.width_ratio
    }
}

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
            workspaces: vec![Workspace::new("Default")],
            active_workspace: 0,
            next_id: 0,
            event_rx,
            event_tx,
            ipc_server,
            socket_path,
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

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let modifiers = ctx.input(|i| i.modifiers);

        // Only process shortcuts when Ctrl (or Cmd on Mac) is held
        if !modifiers.ctrl && !modifiers.command {
            return;
        }

        ctx.input(|i| {
            // Ctrl+N: New terminal
            if i.key_pressed(egui::Key::N) {
                self.create_terminal(ctx);
                let ws = self.active_workspace_mut();
                ws.focused_index = ws.panel_order.len() - 1;
            }

            // Ctrl+W: Close focused terminal
            if i.key_pressed(egui::Key::W) {
                self.close_focused();
            }

            // Ctrl+H: Focus previous
            if i.key_pressed(egui::Key::H) {
                self.focus_prev();
            }

            // Ctrl+L: Focus next
            if i.key_pressed(egui::Key::L) {
                self.focus_next();
            }

            // Ctrl+,: Shrink focused
            if i.key_pressed(egui::Key::Comma) {
                self.shrink_focused();
            }

            // Ctrl+.: Grow focused
            if i.key_pressed(egui::Key::Period) {
                self.grow_focused();
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
            .exact_width(200.0)
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Manse").size(16.0).color(egui::Color32::from_rgb(150, 150, 150)));
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Workspace and terminal list
                let mut clicked_terminal: Option<(usize, usize)> = None; // (workspace_idx, terminal_idx)

                for (ws_idx, ws) in self.workspaces.iter().enumerate() {
                    let is_active_workspace = ws_idx == self.active_workspace;

                    // Workspace name
                    let ws_color = if is_active_workspace {
                        egui::Color32::from_rgb(200, 200, 200)
                    } else {
                        egui::Color32::from_rgb(120, 120, 120)
                    };

                    ui.label(
                        egui::RichText::new(&ws.name)
                            .size(13.0)
                            .color(ws_color)
                    );

                    // Terminals in this workspace (indented)
                    ui.indent(ws_idx, |ui| {
                        for (term_idx, &id) in ws.panel_order.iter().enumerate() {
                            if let Some(panel) = self.panels.get(&id) {
                                let is_focused = is_active_workspace && term_idx == ws.focused_index;
                                let text_color = if is_focused {
                                    egui::Color32::from_rgb(100, 150, 255)
                                } else {
                                    egui::Color32::from_rgb(180, 180, 180)
                                };

                                let response = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(panel.display_title())
                                            .size(12.0)
                                            .color(text_color)
                                    )
                                    .truncate()
                                    .sense(egui::Sense::click())
                                );

                                if response.clicked() {
                                    clicked_terminal = Some((ws_idx, term_idx));
                                }
                            }
                        }
                    });

                    ui.add_space(4.0);
                }

                if let Some((ws_idx, term_idx)) = clicked_terminal {
                    self.active_workspace = ws_idx;
                    self.workspaces[ws_idx].focused_index = term_idx;
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
                            self.render_indicators(ui);
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

                // Create a horizontal layout for terminals
                ui.horizontal(|ui| {
                    ui.set_min_height(padded_height);

                    // Add left padding
                    ui.add_space(padding);

                    // Apply scroll offset
                    ui.add_space(-scroll_offset);

                    // Render each terminal
                    let border_width = 2.0;
                    for (idx, &id) in panel_order.iter().enumerate() {
                        if let Some(panel) = self.panels.get_mut(&id) {
                            let panel_width = panel.pixel_width(viewport_width);
                            let is_focused = idx == focused_index;

                            // Create a frame for the terminal
                            let frame = if is_focused {
                                egui::Frame::NONE
                                    .stroke(egui::Stroke::new(border_width, egui::Color32::from_rgb(100, 150, 255)))
                            } else {
                                egui::Frame::NONE
                            };

                            // Account for border in inner size
                            let inner_width = panel_width - border_width * 2.0;
                            let inner_height = padded_height - border_width * 2.0;

                            frame.show(ui, |ui| {
                                ui.set_min_size(egui::vec2(panel_width, padded_height));
                                ui.set_max_size(egui::vec2(panel_width, padded_height));

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
                });
            });
    }
}

impl App {
    fn render_indicators(&self, ui: &mut egui::Ui) {
        let ws = self.active_workspace();
        let num_panels = ws.panel_order.len();

        // Show terminal count and minimap
        ui.horizontal(|ui| {
            ui.add_space(8.0);

            // Terminal minimap dots
            let dot_radius = 4.0;
            let dot_spacing = 12.0;

            let (response, painter) = ui.allocate_painter(
                egui::vec2(num_panels as f32 * dot_spacing + dot_radius * 2.0, 20.0),
                egui::Sense::hover(),
            );

            let rect = response.rect;
            let y = rect.center().y;

            for i in 0..num_panels {
                let x = rect.left() + dot_radius + (i as f32 * dot_spacing);
                let is_active = i == ws.focused_index;

                let color = if is_active {
                    egui::Color32::from_rgb(100, 150, 255)
                } else {
                    egui::Color32::from_rgb(80, 80, 80)
                };

                painter.circle_filled(egui::pos2(x, y), dot_radius, color);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Terminal info
            ui.label(
                egui::RichText::new(format!("{}/{}", ws.focused_index + 1, num_panels))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(120, 120, 120))
            );

            // Focused terminal title
            if let Some(focused_panel) = self.focused_panel() {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.add(
                    egui::Label::new(
                        egui::RichText::new(focused_panel.display_title())
                            .size(12.0)
                            .color(egui::Color32::from_rgb(180, 180, 180))
                    )
                    .truncate()
                );
            }
        });
    }
}
