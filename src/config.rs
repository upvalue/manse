//! Configuration loading from Lua scripts.
//!
//! Loads `init.lua` from the project root (found by walking up from the executable).

use mlua::{Lua, Result as LuaResult};
use std::path::PathBuf;

/// Sidebar configuration
#[derive(Debug, Clone)]
pub struct SidebarConfig {
    pub width: f32,
    pub workspace_font_size: f32,
    pub terminal_title_font_size: f32,
    pub description_font_size: f32,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            width: 300.0,
            workspace_font_size: 13.0,
            terminal_title_font_size: 12.0,
            description_font_size: 10.0,
        }
    }
}

/// Status bar configuration
#[derive(Debug, Clone)]
pub struct StatusBarConfig {
    pub show_minimap: bool,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self { show_minimap: true }
    }
}

/// A pattern for icon detection
#[derive(Debug, Clone)]
pub struct IconPattern {
    /// Substring to match (case-insensitive)
    pub match_text: String,
    /// Icon to display when matched
    pub icon: String,
}

/// Icon configuration for terminal titles
#[derive(Debug, Clone)]
pub struct IconConfig {
    /// Default icon when no pattern matches
    pub default: String,
    /// Patterns checked in order; first match wins
    pub patterns: Vec<IconPattern>,
}

impl Default for IconConfig {
    fn default() -> Self {
        Self {
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
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub sidebar: SidebarConfig,
    pub status_bar: StatusBarConfig,
    pub terminal_font_size: f32,
    /// Performance logging interval in seconds (0 = disabled)
    pub perf_log_interval: f32,
    /// Icon detection configuration
    pub icons: IconConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sidebar: SidebarConfig::default(),
            status_bar: StatusBarConfig::default(),
            terminal_font_size: 14.0,
            perf_log_interval: 0.0,
            icons: IconConfig::default(),
        }
    }
}

/// Find the project root by walking up from the executable location.
/// Looks for `Cargo.toml` or `init.lua` as markers.
fn find_project_root() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;
    let mut current = exe_path.parent()?;

    // Walk up directory tree looking for project markers
    for _ in 0..10 {
        if current.join("Cargo.toml").exists() || current.join("init.lua").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }

    None
}

/// Load configuration from `init.lua` in the project root.
/// Returns default config if no config file exists or on any error.
pub fn load_config() -> Config {
    let Some(project_root) = find_project_root() else {
        log::debug!("Could not find project root, using default config");
        return Config::default();
    };

    let config_path = project_root.join("init.lua");
    if !config_path.exists() {
        log::debug!("No init.lua found at {}, using default config", config_path.display());
        return Config::default();
    }

    match load_config_from_file(&config_path) {
        Ok(config) => {
            log::info!("Loaded config from {}", config_path.display());
            config
        }
        Err(e) => {
            log::error!("Failed to load config from {}: {}", config_path.display(), e);
            Config::default()
        }
    }
}

/// Load configuration from a specific Lua file.
fn load_config_from_file(path: &PathBuf) -> LuaResult<Config> {
    let lua = Lua::new();

    // Create config table with defaults
    let sidebar_defaults = SidebarConfig::default();
    let status_bar_defaults = StatusBarConfig::default();
    let config_defaults = Config::default();
    lua.load(&format!(
        r#"
        config = {{
            sidebar_width = {sidebar_width},
            workspace_font_size = {workspace_font_size},
            terminal_title_font_size = {terminal_title_font_size},
            description_font_size = {description_font_size},
            terminal_font_size = {terminal_font_size},
            perf_log_interval = {perf_log_interval},
            show_minimap = {show_minimap},
        }}
        "#,
        sidebar_width = sidebar_defaults.width,
        workspace_font_size = sidebar_defaults.workspace_font_size,
        terminal_title_font_size = sidebar_defaults.terminal_title_font_size,
        description_font_size = sidebar_defaults.description_font_size,
        terminal_font_size = config_defaults.terminal_font_size,
        perf_log_interval = config_defaults.perf_log_interval,
        show_minimap = status_bar_defaults.show_minimap,
    ))
    .exec()?;

    // Execute user script
    let script = std::fs::read_to_string(path)
        .map_err(|e| mlua::Error::runtime(format!("Failed to read config file: {}", e)))?;
    lua.load(&script).exec()?;

    // Read values back from the table
    let globals = lua.globals();
    let config_table: mlua::Table = globals.get("config")?;

    // Parse icons config if present, otherwise use defaults
    let icons = if let Ok(icons_table) = config_table.get::<mlua::Table>("icons") {
        let default: String = icons_table
            .get("default")
            .unwrap_or_else(|_| IconConfig::default().default);

        let mut patterns = Vec::new();
        if let Ok(patterns_table) = icons_table.get::<mlua::Table>("patterns") {
            for pair in patterns_table.pairs::<i64, mlua::Table>() {
                if let Ok((_, entry)) = pair {
                    if let (Ok(match_text), Ok(icon)) =
                        (entry.get::<String>("match"), entry.get::<String>("icon"))
                    {
                        patterns.push(IconPattern { match_text, icon });
                    }
                }
            }
        }

        IconConfig { default, patterns }
    } else {
        IconConfig::default()
    };

    let config = Config {
        sidebar: SidebarConfig {
            width: config_table.get("sidebar_width")?,
            workspace_font_size: config_table.get("workspace_font_size")?,
            terminal_title_font_size: config_table.get("terminal_title_font_size")?,
            description_font_size: config_table.get("description_font_size")?,
        },
        status_bar: StatusBarConfig {
            show_minimap: config_table.get("show_minimap")?,
        },
        terminal_font_size: config_table.get("terminal_font_size")?,
        perf_log_interval: config_table.get("perf_log_interval")?,
        icons,
    };

    Ok(config)
}
