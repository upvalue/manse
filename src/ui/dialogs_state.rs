use crate::ui::dialogs;
use eframe::egui;

/// Active dialog type
#[derive(Default)]
pub enum ActiveDialog {
    #[default]
    None,
    /// Confirm close terminal dialog
    ConfirmClose,
    /// Set description input dialog
    SetDescription {
        input: String,
    },
}

pub enum DialogAction {
    None,
    ConfirmClose,
    SaveDescription(String),
}

pub fn render_dialogs(
    ctx: &egui::Context,
    active: &mut ActiveDialog,
    ) -> DialogAction {
    match active {
        ActiveDialog::None => DialogAction::None,
        ActiveDialog::ConfirmClose => match dialogs::render_confirm_close(ctx) {
            dialogs::ConfirmCloseResult::None => DialogAction::None,
            dialogs::ConfirmCloseResult::Cancelled => {
                *active = ActiveDialog::None;
                DialogAction::None
            }
            dialogs::ConfirmCloseResult::Confirmed => {
                *active = ActiveDialog::None;
                DialogAction::ConfirmClose
            }
        },
        ActiveDialog::SetDescription { input } => match dialogs::render_set_description(ctx, input) {
            dialogs::SetDescriptionResult::Open { input } => {
                *active = ActiveDialog::SetDescription { input };
                DialogAction::None
            }
            dialogs::SetDescriptionResult::Cancelled => {
                *active = ActiveDialog::None;
                DialogAction::None
            }
            dialogs::SetDescriptionResult::Saved { description } => {
                *active = ActiveDialog::None;
                DialogAction::SaveDescription(description)
            }
        },
    }
}
