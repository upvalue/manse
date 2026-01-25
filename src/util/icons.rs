/// Icon detection for terminal panels.
///
/// Maps terminal titles to icons based on keyword matching.

/// Default terminal icon
pub const TERMINAL: &str = "🖥️";

/// Neovim icon
pub const NEOVIM: &str = "✏️";

/// Vim icon
pub const VIM: &str = "✏️";

/// Agent icon (Claude Code, etc.)
pub const AGENT: &str = "🤖";

/// Detects an icon from a terminal title.
///
/// Returns an icon if the title matches a known pattern (case-insensitive).
/// Priority order: claude > nvim/neovim > vim
pub fn detect_icon(title: &str) -> Option<&'static str> {
    let title_lower = title.to_lowercase();

    // Claude/agent detection (highest priority)
    if title_lower.contains("claude") {
        return Some(AGENT);
    }

    // Neovim detection (before vim to avoid false matches)
    if title_lower.contains("nvim") || title_lower.contains("neovim") {
        return Some(NEOVIM);
    }

    // Vim detection
    if title_lower.contains("vim") {
        return Some(VIM);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_claude() {
        assert_eq!(detect_icon("Claude Code"), Some(AGENT));
        assert_eq!(detect_icon("claude"), Some(AGENT));
        assert_eq!(detect_icon("CLAUDE"), Some(AGENT));
        assert_eq!(detect_icon("Working with Claude"), Some(AGENT));
    }

    #[test]
    fn detect_neovim() {
        assert_eq!(detect_icon("nvim"), Some(NEOVIM));
        assert_eq!(detect_icon("NVIM"), Some(NEOVIM));
        assert_eq!(detect_icon("neovim"), Some(NEOVIM));
        assert_eq!(detect_icon("Neovim"), Some(NEOVIM));
        assert_eq!(detect_icon("nvim src/main.rs"), Some(NEOVIM));
    }

    #[test]
    fn detect_vim() {
        assert_eq!(detect_icon("vim"), Some(VIM));
        assert_eq!(detect_icon("VIM"), Some(VIM));
        assert_eq!(detect_icon("vim file.txt"), Some(VIM));
    }

    #[test]
    fn nvim_takes_priority_over_vim() {
        // "nvim" contains "vim" but should match nvim
        assert_eq!(detect_icon("nvim"), Some(NEOVIM));
    }

    #[test]
    fn no_match_returns_none() {
        assert_eq!(detect_icon("bash"), None);
        assert_eq!(detect_icon("Terminal"), None);
        assert_eq!(detect_icon("zsh"), None);
        assert_eq!(detect_icon(""), None);
    }

    #[test]
    fn claude_takes_priority() {
        // If both claude and vim are in title, claude wins
        assert_eq!(detect_icon("Claude editing with vim"), Some(AGENT));
    }
}
