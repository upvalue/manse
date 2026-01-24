use crate::config::SidebarConfig;
use crate::icons;
use crate::terminal::TerminalPanel;
use crate::workspace::Workspace;
use eframe::egui;
use std::borrow::Cow;
use std::collections::HashMap;

/// Result of sidebar interaction
pub enum SidebarAction {
    /// A workspace was clicked (switch to it)
    SwitchWorkspace(usize),
    /// A terminal was clicked (switch workspace and focus terminal)
    FocusTerminal { workspace: usize, terminal: usize },
}

/// Build info captured at compile time
pub const BUILD_GIT_HASH: &str = env!("BUILD_GIT_HASH");
pub const BUILD_TIME: &str = env!("BUILD_TIME");

/// Renders the sidebar with workspace and terminal list.
/// Returns an action if a workspace or terminal was clicked.
pub fn render(
    ui: &mut egui::Ui,
    workspaces: &[Workspace],
    active_workspace: usize,
    panels: &HashMap<u64, TerminalPanel>,
    follow_mode: bool,
    config: &SidebarConfig,
) -> Option<SidebarAction> {
    let mut action: Option<SidebarAction> = None;
    let mut global_term_idx: usize = 0;

    // Reserve space for footer at bottom
    let footer_height = 24.0;
    let available = ui.available_height();
    let main_height = available - footer_height;

    // Main scrollable content
    egui::ScrollArea::vertical()
        .max_height(main_height)
        .show(ui, |ui| {
            ui.add_space(10.0);

            for (ws_idx, ws) in workspaces.iter().enumerate() {
                let is_active_workspace = ws_idx == active_workspace;

                // Workspace name (clickable)
                let ws_color = if is_active_workspace {
                    egui::Color32::from_rgb(200, 200, 200)
                } else {
                    egui::Color32::from_rgb(120, 120, 120)
                };

                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    let response = ui.add(
                        egui::Label::new(
                            egui::RichText::new(&ws.name)
                                .size(config.workspace_font_size)
                                .color(ws_color),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if response.clicked() {
                        action = Some(SidebarAction::SwitchWorkspace(ws_idx));
                    }
                });

                // Terminals in this workspace (indented less since icon provides spacing)
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        for (term_idx, &id) in ws.panel_order.iter().enumerate() {
                            if let Some(panel) = panels.get(&id) {
                                let is_focused =
                                    is_active_workspace && term_idx == ws.focused_index;
                                let text_color = if is_focused {
                                    egui::Color32::from_rgb(100, 150, 255)
                                } else {
                                    egui::Color32::from_rgb(180, 180, 180)
                                };

                                // Use custom emoji if set, otherwise auto-detect from title
                                let emoji = panel
                                    .emoji
                                    .as_deref()
                                    .or_else(|| icons::detect_emoji(panel.display_title()));

                                // Title (with optional follow mode letter prefix)
                                // Use Cow to avoid allocation when not in follow mode
                                let title_text: Cow<str> = if follow_mode && global_term_idx < 26 {
                                    let letter = (b'a' + global_term_idx as u8) as char;
                                    Cow::Owned(format!("{} {}", letter, panel.display_title()))
                                } else {
                                    Cow::Borrowed(panel.display_title())
                                };

                                // Render emoji, title, and notification indicator horizontally
                                let response = ui
                                    .horizontal(|ui| {
                                        // Show notification indicator if notified
                                        if panel.notified {
                                            ui.label(
                                                egui::RichText::new("●")
                                                    .size(config.terminal_title_font_size)
                                                    .color(egui::Color32::from_rgb(255, 100, 100)),
                                            );
                                        }

                                        // Show emoji if set, otherwise default terminal icon
                                        let emoji_text = emoji.unwrap_or("🖥️");
                                        ui.label(
                                            egui::RichText::new(emoji_text)
                                                .size(config.terminal_title_font_size),
                                        );

                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&*title_text)
                                                    .size(config.terminal_title_font_size)
                                                    .color(text_color),
                                            )
                                            .truncate()
                                            .sense(egui::Sense::click()),
                                        )
                                    })
                                    .inner;

                                if response.clicked() {
                                    action = Some(SidebarAction::FocusTerminal {
                                        workspace: ws_idx,
                                        terminal: term_idx,
                                    });
                                }

                                // Description (if set)
                                if !panel.description.is_empty() {
                                    let desc_color = if is_focused {
                                        egui::Color32::from_rgb(80, 120, 200)
                                    } else {
                                        egui::Color32::from_rgb(120, 120, 120)
                                    };
                                    let desc_response = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&panel.description)
                                                .size(config.description_font_size)
                                                .color(desc_color),
                                        )
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                    );
                                    if desc_response.clicked() {
                                        action = Some(SidebarAction::FocusTerminal {
                                            workspace: ws_idx,
                                            terminal: term_idx,
                                        });
                                    }
                                }

                                global_term_idx += 1;
                            }
                        }
                    });
                });

                ui.add_space(4.0);
            }
        });

    // Footer with build info
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(format!("{} @ {}", BUILD_GIT_HASH, BUILD_TIME))
                    .size(10.0)
                    .color(egui::Color32::from_rgb(80, 80, 80)),
            );
        });
    });

    action
}
