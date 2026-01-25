use crate::ui::{ActiveDialog, Command};
use crate::util::layout;
use eframe::egui;

use super::App;

impl App {
    pub(crate) fn execute_command(&mut self, cmd: Command, ctx: &egui::Context) {
        match cmd {
            Command::NewTerminal => {
                let ws = self.active_workspace_mut();
                let new_index = ws.focused_index + 1;
                self.create_terminal(ctx);
                self.active_workspace_mut().focused_index = new_index;
            }
            Command::CloseTerminal => {
                self.active_dialog = ActiveDialog::ConfirmClose;
            }
            Command::FocusPrevious => self.focus_prev(),
            Command::FocusNext => self.focus_next(),
            Command::SwapWithPrevious => self.swap_with_prev(),
            Command::SwapWithNext => self.swap_with_next(),
            Command::ShrinkTerminal => self.shrink_focused(),
            Command::GrowTerminal => self.grow_focused(),
            Command::FollowMode => self.follow_mode = true,
            Command::MoveToSpot => self.move_to_spot_mode = true,
            Command::SetDescription => {
                let current = self
                    .focused_panel()
                    .map(|p| p.description.clone())
                    .unwrap_or_default();
                self.active_dialog = ActiveDialog::SetDescription { input: current };
            }
        }
    }

    /// Build a mapping of letter index (0-25) to (workspace_idx, terminal_idx)
    fn build_follow_targets(&self) -> Vec<(usize, usize)> {
        let counts: Vec<usize> = self.workspaces.iter().map(|ws| ws.panel_order.len()).collect();
        layout::build_follow_targets(&counts)
    }

    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if !matches!(self.active_dialog, ActiveDialog::None) {
            return;
        }

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

            let letter_keys = [
                egui::Key::A,
                egui::Key::B,
                egui::Key::C,
                egui::Key::D,
                egui::Key::E,
                egui::Key::F,
                egui::Key::G,
                egui::Key::H,
                egui::Key::I,
                egui::Key::J,
                egui::Key::K,
                egui::Key::L,
                egui::Key::M,
                egui::Key::N,
                egui::Key::O,
                egui::Key::P,
                egui::Key::Q,
                egui::Key::R,
                egui::Key::S,
                egui::Key::T,
                egui::Key::U,
                egui::Key::V,
                egui::Key::W,
                egui::Key::X,
                egui::Key::Y,
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

            return;
        }

        if self.move_to_spot_mode {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.move_to_spot_mode = false;
                return;
            }

            let letter_keys = [
                egui::Key::A,
                egui::Key::B,
                egui::Key::C,
                egui::Key::D,
                egui::Key::E,
                egui::Key::F,
                egui::Key::G,
                egui::Key::H,
                egui::Key::I,
                egui::Key::J,
                egui::Key::K,
                egui::Key::L,
                egui::Key::M,
                egui::Key::N,
                egui::Key::O,
                egui::Key::P,
                egui::Key::Q,
                egui::Key::R,
                egui::Key::S,
                egui::Key::T,
                egui::Key::U,
                egui::Key::V,
                egui::Key::W,
                egui::Key::X,
                egui::Key::Y,
                egui::Key::Z,
            ];

            for (idx, &key) in letter_keys.iter().enumerate() {
                if ctx.input(|i| i.key_pressed(key)) {
                    let targets = self.build_follow_targets();
                    if let Some(&(target_ws, target_idx)) = targets.get(idx) {
                        self.move_focused_to_spot(target_ws, target_idx);
                    }
                    self.move_to_spot_mode = false;
                    return;
                }
            }

            return;
        }

        let modifiers = ctx.input(|i| i.modifiers);

        if modifiers.command && ctx.input(|i| i.key_pressed(egui::Key::P)) {
            self.command_palette_open = !self.command_palette_open;
            return;
        }

        if self.command_palette_open {
            return;
        }

        if !modifiers.command {
            return;
        }

        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::T) {
                self.execute_command(Command::NewTerminal, ctx);
            }

            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::W) {
                self.execute_command(Command::CloseTerminal, ctx);
            }

            if i.key_pressed(egui::Key::OpenCurlyBracket) && i.modifiers.command {
                self.execute_command(Command::SwapWithPrevious, ctx);
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::OpenBracket) {
                self.execute_command(Command::FocusPrevious, ctx);
            }

            if i.key_pressed(egui::Key::CloseCurlyBracket) && i.modifiers.command {
                self.execute_command(Command::SwapWithNext, ctx);
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::CloseBracket) {
                self.execute_command(Command::FocusNext, ctx);
            }

            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus) {
                self.execute_command(Command::ShrinkTerminal, ctx);
            }

            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals) {
                self.execute_command(Command::GrowTerminal, ctx);
            }

            if i.key_pressed(egui::Key::J) && i.modifiers.command && i.modifiers.shift {
                self.execute_command(Command::MoveToSpot, ctx);
            } else if i.consume_key(egui::Modifiers::COMMAND, egui::Key::J) {
                self.execute_command(Command::FollowMode, ctx);
            }

            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::D) {
                self.execute_command(Command::SetDescription, ctx);
            }
        });
    }
}
