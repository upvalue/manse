use crate::ipc_protocol::{Request, Response};
use crate::workspace::Workspace;
use eframe::egui;

use super::App;

impl App {
    pub(crate) fn process_ipc(&mut self, ctx: &egui::Context) {
        let Some(handle) = &self.ipc_handle else {
            return;
        };

        for pending in handle.poll() {
            self.perf_stats.on_ipc_request();
            let response = match pending.request {
                Request::Ping => Response::ok(),
                Request::Restart => {
                    pending.respond(Response::ok());

                    #[cfg(unix)]
                    if let Err(e) = self.trigger_restart() {
                        log::error!("Restart failed: {}", e);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    continue;
                }
                Request::TermRename { ref terminal, ref title } => {
                    let panel = self.panels.values_mut().find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        panel.custom_title = Some(title.clone());
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermDesc {
                    ref terminal,
                    ref description,
                } => {
                    let panel = self.panels.values_mut().find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        panel.cli_description = if description.is_empty() {
                            None
                        } else {
                            Some(description.clone())
                        };
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermIcon { ref terminal, ref icon } => {
                    let panel = self.panels.values_mut().find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        if icon.is_empty() {
                            panel.icon = None;
                        } else {
                            panel.icon = Some(icon.clone());
                        }
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermNotify { ref terminal } => {
                    let panel = self.panels.values_mut().find(|p| p.id == *terminal);

                    if let Some(panel) = panel {
                        panel.notified = true;
                        Response::ok()
                    } else {
                        Response::error(format!("Terminal not found: {}", terminal))
                    }
                }
                Request::TermToWorkspace {
                    ref terminal,
                    ref workspace_name,
                } => {
                    let panel_id = self
                        .panels
                        .iter()
                        .find(|(_, p)| p.id == *terminal)
                        .map(|(&id, _)| id);

                    match panel_id {
                        Some(id) => {
                            let current_ws_idx = self
                                .workspaces
                                .iter()
                                .position(|ws| ws.panel_order.contains(&id));

                            if let Some(ws_idx) = current_ws_idx {
                                if self.workspaces[ws_idx].name == *workspace_name {
                                    self.active_workspace = ws_idx;
                                    pending.respond(Response::ok());
                                    continue;
                                }
                            }

                            for ws in &mut self.workspaces {
                                if let Some(pos) = ws.panel_order.iter().position(|&x| x == id) {
                                    ws.panel_order.remove(pos);
                                    if ws.focused_index >= ws.panel_order.len()
                                        && !ws.panel_order.is_empty()
                                    {
                                        ws.focused_index = ws.panel_order.len() - 1;
                                    }
                                    ws.invalidate_positions();
                                    break;
                                }
                            }

                            let target_ws_idx = self
                                .workspaces
                                .iter()
                                .position(|ws| ws.name == *workspace_name);

                            let target_ws_idx = match target_ws_idx {
                                Some(idx) => idx,
                                None => {
                                    self.workspaces.push(Workspace::new(workspace_name));
                                    self.workspaces.len() - 1
                                }
                            };

                            self.workspaces[target_ws_idx].panel_order.push(id);
                            self.workspaces[target_ws_idx].focused_index =
                                self.workspaces[target_ws_idx].panel_order.len() - 1;
                            self.workspaces[target_ws_idx].invalidate_positions();

                            self.active_workspace = target_ws_idx;
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
