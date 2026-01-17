use crate::ipc::{IpcServer, Request, Response};
use eframe::egui;
use egui_term::{BackendSettings, PtyEvent, TerminalBackend, TerminalView};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

/// Width ratios for terminal panels
const WIDTH_RATIOS: [f32; 4] = [0.333, 0.5, 0.667, 1.0];

/// Scroll animation easing factor
const SCROLL_EASING: f32 = 0.15;

/// A terminal panel in the window manager
struct TerminalPanel {
    id: u64,
    backend: TerminalBackend,
    width_ratio: f32,
}

impl TerminalPanel {
    fn new(id: u64, ctx: &egui::Context, event_tx: Sender<(u64, PtyEvent)>) -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        let settings = BackendSettings {
            shell,
            working_directory: std::env::current_dir().ok(),
            ..Default::default()
        };

        let backend = TerminalBackend::new(id, ctx.clone(), event_tx, settings)
            .expect("Failed to create terminal backend");

        Self {
            id,
            backend,
            width_ratio: 1.0,
        }
    }

    fn pixel_width(&self, viewport_width: f32) -> f32 {
        viewport_width * self.width_ratio
    }
}

/// The scrolling window manager
pub struct App {
    /// Terminal panels
    panels: BTreeMap<u64, TerminalPanel>,
    /// Order of panels (left to right)
    panel_order: Vec<u64>,
    /// Currently focused panel index
    focused_index: usize,
    /// Next panel ID
    next_id: u64,
    /// Event receiver for PTY events
    event_rx: Receiver<(u64, PtyEvent)>,
    /// Event sender for creating new terminals
    event_tx: Sender<(u64, PtyEvent)>,
    /// Current scroll offset (animated)
    scroll_offset: f32,
    /// Target scroll offset
    target_offset: f32,
    /// IPC server for external control
    ipc_server: Option<IpcServer>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, socket_path: Option<PathBuf>) -> Self {
        let (event_tx, event_rx) = mpsc::channel();

        // Initialize IPC server if socket path provided
        let ipc_server = socket_path.and_then(|path| {
            match IpcServer::new(&path) {
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
            panel_order: Vec::new(),
            focused_index: 0,
            next_id: 0,
            event_rx,
            event_tx,
            scroll_offset: 0.0,
            target_offset: 0.0,
            ipc_server,
        };

        // Create initial terminal
        app.create_terminal(&cc.egui_ctx);

        app
    }

    fn create_terminal(&mut self, ctx: &egui::Context) {
        let id = self.next_id;
        self.next_id += 1;

        let panel = TerminalPanel::new(id, ctx, self.event_tx.clone());
        self.panels.insert(id, panel);
        self.panel_order.push(id);
    }

    fn focused_panel(&self) -> Option<&TerminalPanel> {
        self.panel_order
            .get(self.focused_index)
            .and_then(|id| self.panels.get(id))
    }

    fn focused_panel_mut(&mut self) -> Option<&mut TerminalPanel> {
        self.panel_order
            .get(self.focused_index)
            .and_then(|id| self.panels.get_mut(id))
    }

    fn focus_next(&mut self) {
        if self.focused_index < self.panel_order.len().saturating_sub(1) {
            self.focused_index += 1;
        }
    }

    fn focus_prev(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
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
        if self.panel_order.len() <= 1 {
            return; // Don't close the last terminal
        }

        if let Some(&id) = self.panel_order.get(self.focused_index) {
            self.panels.remove(&id);
            self.panel_order.remove(self.focused_index);

            // Adjust focus index
            if self.focused_index >= self.panel_order.len() {
                self.focused_index = self.panel_order.len().saturating_sub(1);
            }
        }
    }

    fn terminal_x_position(&self, index: usize, viewport_width: f32) -> f32 {
        let mut x = 0.0;
        for i in 0..index {
            if let Some(&id) = self.panel_order.get(i) {
                if let Some(panel) = self.panels.get(&id) {
                    x += panel.pixel_width(viewport_width);
                }
            }
        }
        x
    }

    fn total_content_width(&self, viewport_width: f32) -> f32 {
        self.panel_order
            .iter()
            .filter_map(|id| self.panels.get(id))
            .map(|p| p.pixel_width(viewport_width))
            .sum()
    }

    fn scroll_to_focused(&mut self, viewport_width: f32) {
        if self.panel_order.is_empty() {
            return;
        }

        let term_x = self.terminal_x_position(self.focused_index, viewport_width);
        let term_width = self
            .focused_panel()
            .map(|p| p.pixel_width(viewport_width))
            .unwrap_or(0.0);
        let term_right = term_x + term_width;
        let view_right = self.target_offset + viewport_width;

        if term_x < self.target_offset {
            self.target_offset = term_x;
        } else if term_right > view_right {
            self.target_offset = term_right - viewport_width;
        }

        let max_scroll = (self.total_content_width(viewport_width) - viewport_width).max(0.0);
        self.target_offset = self.target_offset.clamp(0.0, max_scroll);
    }

    fn update_scroll(&mut self) {
        let diff = self.target_offset - self.scroll_offset;
        if diff.abs() > 0.5 {
            self.scroll_offset += diff * SCROLL_EASING;
        } else {
            self.scroll_offset = self.target_offset;
        }
    }

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok((id, event)) = self.event_rx.try_recv() {
            match event {
                PtyEvent::Exit => {
                    // Remove the exited terminal
                    if let Some(pos) = self.panel_order.iter().position(|&x| x == id) {
                        self.panels.remove(&id);
                        self.panel_order.remove(pos);

                        if self.panel_order.is_empty() {
                            // Last terminal closed, exit app
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            return;
                        }

                        // Adjust focus
                        if self.focused_index >= self.panel_order.len() {
                            self.focused_index = self.panel_order.len().saturating_sub(1);
                        }
                    }
                }
                PtyEvent::Title(_title) => {
                    // Could update window title here
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
                self.focused_index = self.panel_order.len() - 1;
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
        for (client_idx, request) in requests {
            let response = match request {
                Request::Ping => Response::ok(),
            };
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

                // Terminal area
                let available = ui.available_size();
                let viewport_width = available.x;

                // Scroll to focused terminal
                self.scroll_to_focused(viewport_width);

                // Create a horizontal layout for terminals
                ui.horizontal(|ui| {
                    ui.set_min_height(available.y);

                    // Apply scroll offset
                    ui.add_space(-self.scroll_offset);

                    // Render each terminal
                    for (idx, &id) in self.panel_order.clone().iter().enumerate() {
                        if let Some(panel) = self.panels.get_mut(&id) {
                            let panel_width = panel.pixel_width(viewport_width);
                            let is_focused = idx == self.focused_index;

                            // Create a frame for the terminal
                            let frame = if is_focused {
                                egui::Frame::NONE
                                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)))
                            } else {
                                egui::Frame::NONE
                            };

                            frame.show(ui, |ui| {
                                ui.set_min_size(egui::vec2(panel_width, available.y));
                                ui.set_max_size(egui::vec2(panel_width, available.y));

                                // Render terminal
                                let term_view = TerminalView::new(ui, &mut panel.backend)
                                    .set_focus(is_focused)
                                    .set_size(egui::vec2(panel_width, available.y));
                                let response = ui.add(term_view);

                                // Ensure the focused terminal has egui focus
                                if is_focused && !response.has_focus() {
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
        let num_panels = self.panel_order.len();

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
                let is_active = i == self.focused_index;

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
                egui::RichText::new(format!("{}/{}", self.focused_index + 1, num_panels))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(120, 120, 120))
            );
        });
    }
}
