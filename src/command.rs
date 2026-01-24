/// A command available in the command palette
#[derive(Clone, Copy, PartialEq)]
pub enum Command {
    NewTerminal,
    CloseTerminal,
    FocusPrevious,
    FocusNext,
    SwapWithPrevious,
    SwapWithNext,
    ShrinkTerminal,
    GrowTerminal,
    FollowMode,
    SetDescription,
}

impl Command {
    /// Returns all commands that should be shown in the command palette
    pub fn all() -> &'static [Command] {
        &[
            Command::NewTerminal,
            Command::CloseTerminal,
            Command::FocusPrevious,
            Command::FocusNext,
            Command::SwapWithPrevious,
            Command::SwapWithNext,
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
            Command::ShrinkTerminal => "⌘-",
            Command::GrowTerminal => "⌘=",
            Command::FollowMode => "⌘J",
            Command::SetDescription => "⌘D",
        }
    }
}
