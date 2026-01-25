/// Command palette UI and command definitions.

use eframe::egui;

/// A command available in the command palette.
#[derive(Clone, Copy, PartialEq)]
pub enum Command {
    NewTerminal,
    CloseTerminal,
    FocusPrevious,
    FocusNext,
    SwapWithPrevious,
    SwapWithNext,
    MoveToSpot,
    ShrinkTerminal,
    GrowTerminal,
    FollowMode,
    SetDescription,
}

impl Command {
    /// Returns all commands that should be shown in the command palette.
    pub fn all() -> &'static [Command] {
        &[
            Command::NewTerminal,
            Command::CloseTerminal,
            Command::FocusPrevious,
            Command::FocusNext,
            Command::SwapWithPrevious,
            Command::SwapWithNext,
            Command::MoveToSpot,
            Command::ShrinkTerminal,
            Command::GrowTerminal,
            Command::FollowMode,
            Command::SetDescription,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Command::NewTerminal => "New Terminal",
            Command::CloseTerminal => "Close Terminal",
            Command::FocusPrevious => "Focus Previous Terminal",
            Command::FocusNext => "Focus Next Terminal",
            Command::SwapWithPrevious => "Swap with Previous Terminal",
            Command::SwapWithNext => "Swap with Next Terminal",
            Command::MoveToSpot => "Move to Spot",
            Command::ShrinkTerminal => "Shrink Terminal",
            Command::GrowTerminal => "Grow Terminal",
            Command::FollowMode => "Follow Mode",
            Command::SetDescription => "Set Terminal Description",
        }
    }

    pub fn keybinding(&self) -> &'static str {
        match self {
            Command::NewTerminal => "⌘T",
            Command::CloseTerminal => "⌘W",
            Command::FocusPrevious => "⌘[",
            Command::FocusNext => "⌘]",
            Command::SwapWithPrevious => "⌘⇧[",
            Command::SwapWithNext => "⌘⇧]",
            Command::MoveToSpot => "⌘⇧J",
            Command::ShrinkTerminal => "⌘-",
            Command::GrowTerminal => "⌘=",
            Command::FollowMode => "⌘J",
            Command::SetDescription => "⌘D",
        }
    }
}

/// Result of rendering the command palette.
pub struct CommandPaletteResult {
    /// Whether the background was clicked (should close palette)
    pub background_clicked: bool,
    /// Command that was selected (if any)
    pub selected_command: Option<Command>,
}

/// Renders the command palette overlay.
/// Returns the result indicating if background was clicked or a command was selected.
pub fn render(ctx: &egui::Context) -> CommandPaletteResult {
    let mut result = CommandPaletteResult {
        background_clicked: false,
        selected_command: None,
    };

    // Semi-transparent background
    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();

    egui::Area::new(egui::Id::new("command_palette_bg"))
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_black_alpha(128),
            );
            if response.clicked() {
                result.background_clicked = true;
            }
        });

    // Command palette window
    let palette_width = 400.0;
    let palette_x = (screen_rect.width() - palette_width) / 2.0;
    let palette_y = screen_rect.height() * 0.2;

    egui::Area::new(egui::Id::new("command_palette"))
        .fixed_pos(egui::pos2(palette_x, palette_y))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgb(40, 40, 40))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_width(palette_width);
                    ui.add_space(8.0);

                    // Title
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Command Palette")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(180, 180, 180)),
                        );
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Command list
                    for cmd in Command::all() {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(palette_width - 16.0, 28.0),
                            egui::Sense::click(),
                        );

                        // Paint hover background first (before text)
                        if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                4.0,
                                egui::Color32::from_rgb(60, 60, 60),
                            );
                        }

                        // Then paint the text on top
                        ui.painter().text(
                            rect.left_center() + egui::vec2(8.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{}  {}", cmd.name(), cmd.keybinding()),
                            egui::FontId::proportional(13.0),
                            egui::Color32::from_rgb(220, 220, 220),
                        );

                        if response.clicked() {
                            result.selected_command = Some(*cmd);
                        }
                    }

                    ui.add_space(8.0);
                });
        });

    result
}
