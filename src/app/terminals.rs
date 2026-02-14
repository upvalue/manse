use crate::terminal::TerminalPanel;
use crate::util::layout;
use crate::workspace::Workspace;
use eframe::egui;
use egui_term::PtyEvent;
use std::path::PathBuf;

use super::App;
use super::WIDTH_RATIOS;

impl App {
    pub(crate) fn active_workspace(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    pub(crate) fn active_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// Remove empty workspaces except "default". Adjusts active_workspace index if needed.
    pub(crate) fn cleanup_empty_workspaces(&mut self) {
        let mut i = 0;
        while i < self.workspaces.len() {
            if self.workspaces[i].panel_order.is_empty() && self.workspaces[i].name != "default" {
                self.workspaces.remove(i);
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

    pub(crate) fn create_terminal(&mut self, ctx: &egui::Context) {
        let id = self.next_id;
        self.next_id += 1;

        let working_dir = self
            .focused_panel()
            .and_then(|p| p.current_working_directory.clone());

        let panel = TerminalPanel::new(
            id,
            ctx,
            self.event_tx.clone(),
            self.socket_path.as_ref(),
            working_dir,
        );
        self.panels.insert(id, panel);

        let ws = self.active_workspace_mut();
        if ws.panel_order.is_empty() {
            ws.panel_order.push(id);
        } else {
            let insert_pos = ws.focused_index + 1;
            ws.panel_order.insert(insert_pos, id);
        }
        ws.invalidate_positions();
    }

    pub(crate) fn focused_panel(&self) -> Option<&TerminalPanel> {
        let ws = self.active_workspace();
        ws.panel_order
            .get(ws.focused_index)
            .and_then(|id| self.panels.get(id))
    }

    pub(crate) fn focused_panel_mut(&mut self) -> Option<&mut TerminalPanel> {
        let focused_id = self
            .active_workspace()
            .panel_order
            .get(self.active_workspace().focused_index)
            .copied();
        focused_id.and_then(|id| self.panels.get_mut(&id))
    }

    pub(crate) fn focus_next(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index < ws.panel_order.len().saturating_sub(1) {
            ws.focused_index += 1;
        }
        self.log_ssh_status();
    }

    pub(crate) fn focus_prev(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index > 0 {
            ws.focused_index -= 1;
        }
        self.log_ssh_status();
    }

    /// Log whether the currently focused terminal is running an SSH session.
    pub(crate) fn log_ssh_status(&self) {
        if let Some(panel) = self.focused_panel() {
            match panel.detect_ssh() {
                Some(ssh) => {
                    log::info!(
                        "Terminal {} is SSH'd to {} (full cmd: {})",
                        panel.display_title(),
                        ssh,
                        ssh.full_command,
                    );
                }
                None => {
                    log::debug!(
                        "Terminal {} — no SSH session detected",
                        panel.display_title(),
                    );
                }
            }
        }
    }

    pub(crate) fn grow_focused(&mut self) {
        if let Some(panel) = self.focused_panel_mut() {
            if let Some(new_ratio) = layout::next_ratio(&WIDTH_RATIOS, panel.width_ratio, 0.01) {
                panel.width_ratio = new_ratio;
            }
        }
        self.active_workspace_mut().invalidate_positions();
    }

    pub(crate) fn shrink_focused(&mut self) {
        if let Some(panel) = self.focused_panel_mut() {
            if let Some(new_ratio) = layout::prev_ratio(&WIDTH_RATIOS, panel.width_ratio, 0.01) {
                panel.width_ratio = new_ratio;
            }
        }
        self.active_workspace_mut().invalidate_positions();
    }

    pub(crate) fn swap_with_prev(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index > 0 {
            ws.panel_order.swap(ws.focused_index, ws.focused_index - 1);
            ws.focused_index -= 1;
            ws.invalidate_positions();
        }
    }

    pub(crate) fn swap_with_next(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.focused_index < ws.panel_order.len().saturating_sub(1) {
            ws.panel_order.swap(ws.focused_index, ws.focused_index + 1);
            ws.focused_index += 1;
            ws.invalidate_positions();
        }
    }

    /// Move the currently focused terminal to a specific spot identified by workspace and index.
    /// The terminal is inserted *before* the terminal currently at that position.
    pub(crate) fn move_focused_to_spot(&mut self, target_ws: usize, target_idx: usize) {
        let source_ws = self.active_workspace;
        let source_idx = self.workspaces[source_ws].focused_index;

        let Some(&panel_id) = self.workspaces[source_ws].panel_order.get(source_idx) else {
            return;
        };

        if source_ws == target_ws && source_idx == target_idx {
            return;
        }

        if source_ws == target_ws {
            let ws = &mut self.workspaces[source_ws];
            ws.panel_order.remove(source_idx);

            let adjusted_idx = if source_idx < target_idx {
                target_idx - 1
            } else {
                target_idx
            };

            ws.panel_order.insert(adjusted_idx, panel_id);
            ws.focused_index = adjusted_idx;
            ws.invalidate_positions();
        } else {
            self.workspaces[source_ws].panel_order.remove(source_idx);
            let source_len = self.workspaces[source_ws].panel_order.len();
            if source_len == 0 {
                self.workspaces[source_ws].focused_index = 0;
            } else if self.workspaces[source_ws].focused_index >= source_len {
                self.workspaces[source_ws].focused_index = source_len - 1;
            }
            self.workspaces[source_ws].invalidate_positions();

            let target_len = self.workspaces[target_ws].panel_order.len();
            let insert_idx = target_idx.min(target_len);
            self.workspaces[target_ws].panel_order.insert(insert_idx, panel_id);
            self.workspaces[target_ws].focused_index = insert_idx;
            self.workspaces[target_ws].invalidate_positions();

            self.active_workspace = target_ws;
        }
    }

    pub(crate) fn close_focused(&mut self) {
        let ws = self.active_workspace_mut();
        if ws.panel_order.len() <= 1 {
            return;
        }

        if let Some(&id) = ws.panel_order.get(ws.focused_index) {
            self.panels.remove(&id);
            let ws = self.active_workspace_mut();
            ws.panel_order.remove(ws.focused_index);

            if ws.focused_index >= ws.panel_order.len() {
                ws.focused_index = ws.panel_order.len().saturating_sub(1);
            }
            ws.invalidate_positions();
        }
    }

    /// Compute and cache terminal positions for the active workspace.
    pub(crate) fn ensure_positions_cached(&mut self, viewport_width: f32) {
        let ws = self.active_workspace();
        if (ws.cached_positions.viewport_width - viewport_width).abs() < 0.1
            && ws.cached_positions.positions.len() == ws.panel_order.len()
        {
            return;
        }

        let panel_order: Vec<u64> = ws.panel_order.clone();
        let widths: Vec<f32> = panel_order
            .iter()
            .filter_map(|id| self.panels.get(id).map(|p| p.pixel_width(viewport_width)))
            .collect();

        let raw_positions = layout::compute_positions(widths.into_iter());

        let positions: Vec<(u64, f32, f32)> = panel_order
            .into_iter()
            .zip(raw_positions)
            .map(|(id, (x, w))| (id, x, w))
            .collect();

        let ws = self.active_workspace_mut();
        ws.cached_positions.positions = positions;
        ws.cached_positions.viewport_width = viewport_width;
    }

    pub(crate) fn scroll_to_focused(&mut self, viewport_width: f32) {
        let ws = self.active_workspace();
        if ws.panel_order.is_empty() {
            return;
        }

        let focused_index = ws.focused_index;
        let current_target = ws.target_offset;

        let positions: Vec<(f32, f32)> = ws
            .cached_positions
            .positions
            .iter()
            .map(|&(_, x, w)| (x, w))
            .collect();

        let new_target = layout::scroll_target_for_visible(
            &positions,
            focused_index,
            current_target,
            viewport_width,
        );

        self.active_workspace_mut().target_offset = new_target;
    }

    pub(crate) fn update_scroll(&mut self) {
        let ws = self.active_workspace_mut();
        ws.scroll_offset = layout::ease_toward(
            ws.scroll_offset,
            ws.target_offset,
            layout::SCROLL_EASING,
        );
    }

    pub(crate) fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok((id, event)) = self.event_rx.try_recv() {
            self.perf_stats.on_pty_event();
            match event {
                PtyEvent::Exit => {
                    for ws in &mut self.workspaces {
                        if let Some(pos) = ws.panel_order.iter().position(|&x| x == id) {
                            ws.panel_order.remove(pos);

                            if ws.focused_index >= ws.panel_order.len() {
                                ws.focused_index = ws.panel_order.len().saturating_sub(1);
                            }
                            ws.invalidate_positions();
                            break;
                        }
                    }

                    self.panels.remove(&id);
                    self.cleanup_empty_workspaces();

                    let total_terminals: usize =
                        self.workspaces.iter().map(|ws| ws.panel_order.len()).sum();
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
}
