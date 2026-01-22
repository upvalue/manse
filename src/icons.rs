/// Emoji icons for terminal panels

/// Detects an emoji icon from a terminal title
pub fn detect_emoji(title: &str) -> Option<&'static str> {
    let title_lower = title.to_lowercase();

    // Neovim detection - edit emoji
    if title_lower.contains("nvim") || title_lower.contains("neovim") {
        return Some("✏️");
    }

    None
}
