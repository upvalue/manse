use crate::config::{IconConfig, SidebarConfig, UiConfig};
use crate::terminal::TerminalPanel;
use crate::util::icons;
use crate::util::layout;
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
    show_jump_letters: bool,
    config: &SidebarConfig,
    icons: &IconConfig,
    ui_colors: &UiConfig,
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
                    ui_colors.sidebar_text
                } else {
                    ui_colors.sidebar_text_dim
                };

                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    let response = ui.add(
                        egui::Label::new(
                            egui::RichText::new(&ws.name)
                                .size(config.workspace_font_size + 2.0)
                                .strong()
                                .color(ws_color),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if response.clicked() {
                        action = Some(SidebarAction::SwitchWorkspace(ws_idx));
                    }
                });

                ui.add_space(4.0);

                // Terminals in this workspace (indented under workspace header)
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        for (term_idx, &id) in ws.panel_order.iter().enumerate() {
                            if let Some(panel) = panels.get(&id) {
                                let is_focused =
                                    is_active_workspace && term_idx == ws.focused_index;
                                let text_color = if is_focused {
                                    ui_colors.focused_border
                                } else {
                                    ui_colors.sidebar_text
                                };

                                // Use custom icon if set, otherwise auto-detect from title
                                let icon: &str = panel.icon.as_deref().unwrap_or_else(|| {
                                    icons::detect_icon(panel.display_title(), icons)
                                });

                                // Title (with optional follow mode letter prefix)
                                // Use Cow to avoid allocation when not in follow mode
                                let title_text: Cow<str> = if show_jump_letters {
                                    if let Some(letter) = layout::index_to_letter(global_term_idx) {
                                        Cow::Owned(format!("{} {}", letter, panel.display_title()))
                                    } else {
                                        Cow::Borrowed(panel.display_title())
                                    }
                                } else {
                                    Cow::Borrowed(panel.display_title())
                                };

                                // Background color for notified terminals (dark reddish)
                                let bg_color = if panel.notified {
                                    Some(egui::Color32::from_rgb(60, 25, 25))
                                } else {
                                    None
                                };

                                // Wrap terminal entry in a frame if notified
                                let frame = egui::Frame::new()
                                    .fill(bg_color.unwrap_or(egui::Color32::TRANSPARENT))
                                    .inner_margin(egui::Margin::symmetric(2, 1))
                                    .corner_radius(4.0);

                                let frame_response = frame.show(ui, |ui| {
                                    // Determine if we have a user-set description
                                    let has_description = !panel.description.is_empty();
                                    let has_cli_description = panel.cli_description.is_some();
                                    let has_any_description =
                                        has_description || has_cli_description;

                                    // Primary text: description if set, otherwise title
                                    let primary_text: Cow<str> = if has_description {
                                        // Use in-app description as primary
                                        if show_jump_letters {
                                            if let Some(letter) =
                                                layout::index_to_letter(global_term_idx)
                                            {
                                                Cow::Owned(format!(
                                                    "{} {}",
                                                    letter, &panel.description
                                                ))
                                            } else {
                                                Cow::Borrowed(&panel.description)
                                            }
                                        } else {
                                            Cow::Borrowed(&panel.description)
                                        }
                                    } else if has_cli_description {
                                        // Use CLI description as primary
                                        let cli_desc = panel.cli_description.as_ref().unwrap();
                                        if show_jump_letters {
                                            if let Some(letter) =
                                                layout::index_to_letter(global_term_idx)
                                            {
                                                Cow::Owned(format!("{} {}", letter, cli_desc))
                                            } else {
                                                Cow::Borrowed(cli_desc.as_str())
                                            }
                                        } else {
                                            Cow::Borrowed(cli_desc.as_str())
                                        }
                                    } else {
                                        // No description, use title as primary
                                        title_text.clone()
                                    };

                                    // Render icon and primary text horizontally
                                    let response = ui
                                        .horizontal(|ui| {
                                            // Show icon in fixed-width container for uniform alignment
                                            let icon_text = icon;
                                            let icon_width = config.terminal_title_font_size * 1.5;
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(
                                                    icon_width,
                                                    ui.spacing().interact_size.y,
                                                ),
                                                egui::Layout::centered_and_justified(
                                                    egui::Direction::LeftToRight,
                                                ),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(icon_text)
                                                            .size(config.terminal_title_font_size),
                                                    );
                                                },
                                            );

                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&*primary_text)
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

                                    // If we have a description, show title as secondary (subdued)
                                    if has_any_description {
                                        let secondary_color = if is_focused {
                                            ui_colors.focused_border
                                        } else {
                                            ui_colors.sidebar_text_dim
                                        };
                                        let title_response = ui
                                            .horizontal(|ui| {
                                                // Indent to align with text after icon
                                                ui.add_space(
                                                    config.terminal_title_font_size * 1.5 + 4.0,
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(panel.display_title())
                                                            .size(config.description_font_size)
                                                            .color(secondary_color),
                                                    )
                                                    .truncate()
                                                    .sense(egui::Sense::click()),
                                                )
                                            })
                                            .inner;
                                        if title_response.clicked() {
                                            action = Some(SidebarAction::FocusTerminal {
                                                workspace: ws_idx,
                                                terminal: term_idx,
                                            });
                                        }
                                    }

                                    // Show CLI description as tertiary if in-app description is also set
                                    if has_description && has_cli_description {
                                        let cli_desc = panel.cli_description.as_ref().unwrap();
                                        let desc_color = if is_focused {
                                            ui_colors.focused_border
                                        } else {
                                            ui_colors.sidebar_text_dim
                                        };
                                        let desc_response = ui
                                            .horizontal(|ui| {
                                                // Indent to align with text after icon
                                                ui.add_space(
                                                    config.terminal_title_font_size * 1.5 + 4.0,
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(cli_desc)
                                                            .size(config.description_font_size)
                                                            .color(desc_color),
                                                    )
                                                    .truncate()
                                                    .sense(egui::Sense::click()),
                                                )
                                            })
                                            .inner;
                                        if desc_response.clicked() {
                                            action = Some(SidebarAction::FocusTerminal {
                                                workspace: ws_idx,
                                                terminal: term_idx,
                                            });
                                        }
                                    }
                                });

                                // Also make the frame background clickable
                                if frame_response.response.clicked() {
                                    action = Some(SidebarAction::FocusTerminal {
                                        workspace: ws_idx,
                                        terminal: term_idx,
                                    });
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
                    .color(ui_colors.sidebar_text_dim),
            );
        });
    });

    action
}
