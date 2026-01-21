/// A command available in the command palette
#[derive(Clone, Copy, PartialEq)]
pub enum Command {
    NewTerminal,
    CloseTerminal,
    FocusPrevious,
    FocusNext,
    ShrinkTerminal,
    GrowTerminal,
    FollowMode,
}

impl Command {
    /// Returns all commands that should be shown in the command palette
    pub fn all() -> &'static [Command] {
        &[
            Command::NewTerminal,
            Command::CloseTerminal,
            Command::FocusPrevious,
            Command::FocusNext,
            Command::ShrinkTerminal,
            Command::GrowTerminal,
            Command::FollowMode,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Command::NewTerminal => "New Terminal",
            Command::CloseTerminal => "Close Terminal",
            Command::FocusPrevious => "Focus Previous Terminal",
            Command::FocusNext => "Focus Next Terminal",
            Command::ShrinkTerminal => "Shrink Terminal",
            Command::GrowTerminal => "Grow Terminal",
            Command::FollowMode => "Follow Mode",
        }
    }

    pub fn keybinding(&self) -> &'static str {
        match self {
            Command::NewTerminal => "Ctrl+N",
            Command::CloseTerminal => "Ctrl+W",
            Command::FocusPrevious => "Ctrl+H",
            Command::FocusNext => "Ctrl+L",
            Command::ShrinkTerminal => "Ctrl+,",
            Command::GrowTerminal => "Ctrl+.",
            Command::FollowMode => "",
        }
    }
}
