/// Icon detection for terminal panels.
///
/// Maps terminal titles to icons based on configurable pattern matching.

use crate::config::IconConfig;

/// Detects an icon from a terminal title using the provided config.
///
/// Checks patterns in order; returns the first match.
/// Falls back to the default icon if no pattern matches.
pub fn detect_icon<'a>(title: &str, config: &'a IconConfig) -> &'a str {
    let title_lower = title.to_lowercase();

    for pattern in &config.patterns {
        if title_lower.contains(&pattern.match_text) {
            return &pattern.icon;
        }
    }

    &config.default
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IconPattern;

    fn test_config() -> IconConfig {
        IconConfig {
            default: "🖥️".into(),
            patterns: vec![
                IconPattern {
                    match_text: "claude".into(),
                    icon: "🤖".into(),
                },
                IconPattern {
                    match_text: "nvim".into(),
                    icon: "✏️".into(),
                },
                IconPattern {
                    match_text: "neovim".into(),
                    icon: "✏️".into(),
                },
            ],
        }
    }

    #[test]
    fn detect_claude() {
        let config = test_config();
        assert_eq!(detect_icon("Claude Code", &config), "🤖");
        assert_eq!(detect_icon("claude", &config), "🤖");
        assert_eq!(detect_icon("CLAUDE", &config), "🤖");
        assert_eq!(detect_icon("Working with Claude", &config), "🤖");
    }

    #[test]
    fn detect_neovim() {
        let config = test_config();
        assert_eq!(detect_icon("nvim", &config), "✏️");
        assert_eq!(detect_icon("NVIM", &config), "✏️");
        assert_eq!(detect_icon("neovim", &config), "✏️");
        assert_eq!(detect_icon("Neovim", &config), "✏️");
        assert_eq!(detect_icon("nvim src/main.rs", &config), "✏️");
    }

    #[test]
    fn no_match_returns_default() {
        let config = test_config();
        assert_eq!(detect_icon("bash", &config), "🖥️");
        assert_eq!(detect_icon("Terminal", &config), "🖥️");
        assert_eq!(detect_icon("zsh", &config), "🖥️");
        assert_eq!(detect_icon("", &config), "🖥️");
    }

    #[test]
    fn pattern_order_matters() {
        // First matching pattern wins
        let config = IconConfig {
            default: "🖥️".into(),
            patterns: vec![
                IconPattern {
                    match_text: "special".into(),
                    icon: "⭐".into(),
                },
                IconPattern {
                    match_text: "special".into(),
                    icon: "❌".into(),
                },
            ],
        };
        assert_eq!(detect_icon("special case", &config), "⭐");
    }

    #[test]
    fn custom_patterns() {
        let config = IconConfig {
            default: "📦".into(),
            patterns: vec![
                IconPattern {
                    match_text: "docker".into(),
                    icon: "🐳".into(),
                },
                IconPattern {
                    match_text: "python".into(),
                    icon: "🐍".into(),
                },
            ],
        };
        assert_eq!(detect_icon("docker compose up", &config), "🐳");
        assert_eq!(detect_icon("python script.py", &config), "🐍");
        assert_eq!(detect_icon("cargo build", &config), "📦");
    }
}
