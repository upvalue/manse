pub mod command_palette;
pub mod dialogs;
pub mod dialogs_state;
pub mod sidebar;
pub mod status_bar;
pub mod terminal_strip;

// Re-export Command for convenience
pub use command_palette::Command;
pub use dialogs_state::ActiveDialog;
pub use dialogs_state::DialogAction;
