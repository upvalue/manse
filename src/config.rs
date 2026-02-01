//! Configuration loading from Lua scripts.
//!
//! Loads `init.lua` from the project root (found by walking up from the executable).

use eframe::egui::Color32;
use egui_term::{ColorPalette, TerminalTheme};
use mlua::{Lua, Result as LuaResult};
use std::path::PathBuf;

/// Parse a hex color string like "#1e2132" to Color32
pub fn hex_to_color32(hex: &str) -> Option<Color32> {
    if hex.len() == 7 && hex.starts_with('#') {
        let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
        let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
        let b = u8::from_str_radix(&hex[5..7], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}

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

/// Terminal color scheme configuration.
/// All fields are optional - unset colors use defaults.
#[derive(Debug, Clone, Default)]
pub struct ColorsConfig {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub black: Option<String>,
    pub red: Option<String>,
    pub green: Option<String>,
    pub yellow: Option<String>,
    pub blue: Option<String>,
    pub magenta: Option<String>,
    pub cyan: Option<String>,
    pub white: Option<String>,
    pub bright_black: Option<String>,
    pub bright_red: Option<String>,
    pub bright_green: Option<String>,
    pub bright_yellow: Option<String>,
    pub bright_blue: Option<String>,
    pub bright_magenta: Option<String>,
    pub bright_cyan: Option<String>,
    pub bright_white: Option<String>,
    // Dim colors are auto-derived if not specified
    pub dim_foreground: Option<String>,
    pub dim_black: Option<String>,
    pub dim_red: Option<String>,
    pub dim_green: Option<String>,
    pub dim_yellow: Option<String>,
    pub dim_blue: Option<String>,
    pub dim_magenta: Option<String>,
    pub dim_cyan: Option<String>,
    pub dim_white: Option<String>,
}

impl ColorsConfig {
    /// Build a ColorPalette from this config, using defaults for unset values.
    pub fn build_palette(&self) -> ColorPalette {
        let defaults = ColorPalette::default();

        // Helper to derive a dim color by darkening the base color
        fn derive_dim(hex: &str) -> String {
            // Parse hex color and multiply by 0.65 to darken
            if hex.len() == 7 && hex.starts_with('#') {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[1..3], 16),
                    u8::from_str_radix(&hex[3..5], 16),
                    u8::from_str_radix(&hex[5..7], 16),
                ) {
                    let dr = ((r as f32) * 0.65) as u8;
                    let dg = ((g as f32) * 0.65) as u8;
                    let db = ((b as f32) * 0.65) as u8;
                    return format!("#{:02x}{:02x}{:02x}", dr, dg, db);
                }
            }
            hex.to_string()
        }

        // Get base colors first (for deriving dims)
        let foreground = self.foreground.clone().unwrap_or(defaults.foreground.clone());
        let black = self.black.clone().unwrap_or(defaults.black.clone());
        let red = self.red.clone().unwrap_or(defaults.red.clone());
        let green = self.green.clone().unwrap_or(defaults.green.clone());
        let yellow = self.yellow.clone().unwrap_or(defaults.yellow.clone());
        let blue = self.blue.clone().unwrap_or(defaults.blue.clone());
        let magenta = self.magenta.clone().unwrap_or(defaults.magenta.clone());
        let cyan = self.cyan.clone().unwrap_or(defaults.cyan.clone());
        let white = self.white.clone().unwrap_or(defaults.white.clone());

        ColorPalette {
            foreground: foreground.clone(),
            background: self.background.clone().unwrap_or(defaults.background),
            black: black.clone(),
            red: red.clone(),
            green: green.clone(),
            yellow: yellow.clone(),
            blue: blue.clone(),
            magenta: magenta.clone(),
            cyan: cyan.clone(),
            white: white.clone(),
            bright_black: self.bright_black.clone().unwrap_or(defaults.bright_black),
            bright_red: self.bright_red.clone().unwrap_or(defaults.bright_red),
            bright_green: self.bright_green.clone().unwrap_or(defaults.bright_green),
            bright_yellow: self.bright_yellow.clone().unwrap_or(defaults.bright_yellow),
            bright_blue: self.bright_blue.clone().unwrap_or(defaults.bright_blue),
            bright_magenta: self.bright_magenta.clone().unwrap_or(defaults.bright_magenta),
            bright_cyan: self.bright_cyan.clone().unwrap_or(defaults.bright_cyan),
            bright_white: self.bright_white.clone().unwrap_or(defaults.bright_white),
            bright_foreground: None,
            // Derive dim colors from base colors if not explicitly set
            dim_foreground: self.dim_foreground.clone().unwrap_or_else(|| derive_dim(&foreground)),
            dim_black: self.dim_black.clone().unwrap_or_else(|| derive_dim(&black)),
            dim_red: self.dim_red.clone().unwrap_or_else(|| derive_dim(&red)),
            dim_green: self.dim_green.clone().unwrap_or_else(|| derive_dim(&green)),
            dim_yellow: self.dim_yellow.clone().unwrap_or_else(|| derive_dim(&yellow)),
            dim_blue: self.dim_blue.clone().unwrap_or_else(|| derive_dim(&blue)),
            dim_magenta: self.dim_magenta.clone().unwrap_or_else(|| derive_dim(&magenta)),
            dim_cyan: self.dim_cyan.clone().unwrap_or_else(|| derive_dim(&cyan)),
            dim_white: self.dim_white.clone().unwrap_or_else(|| derive_dim(&white)),
        }
    }
}

/// UI color configuration for Manse's chrome (sidebar, status bar, etc.)
#[derive(Debug, Clone)]
pub struct UiConfig {
    pub sidebar_background: Color32,
    pub sidebar_text: Color32,
    pub sidebar_text_dim: Color32,
    pub status_bar_background: Color32,
    pub status_bar_text: Color32,
    pub focused_border: Color32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_background: Color32::from_rgb(30, 30, 30),
            sidebar_text: Color32::from_rgb(200, 200, 200),
            sidebar_text_dim: Color32::from_rgb(120, 120, 120),
            status_bar_background: Color32::from_rgb(20, 20, 20),
            status_bar_text: Color32::from_rgb(120, 120, 120),
            focused_border: Color32::from_rgb(100, 150, 255),
        }
    }
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub sidebar: SidebarConfig,
    pub status_bar: StatusBarConfig,
    pub terminal_font_size: f32,
    /// Horizontal interior padding inside each terminal panel (pixels)
    pub terminal_padding_x: f32,
    /// Vertical interior padding inside each terminal panel (pixels)
    pub terminal_padding_y: f32,
    /// Performance logging interval in seconds (0 = disabled)
    pub perf_log_interval: f32,
    /// Icon detection configuration
    pub icons: IconConfig,
    /// Terminal color scheme
    pub colors: ColorsConfig,
    /// UI colors (sidebar, status bar, borders)
    pub ui_colors: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sidebar: SidebarConfig::default(),
            status_bar: StatusBarConfig::default(),
            terminal_font_size: 14.0,
            terminal_padding_x: 8.0,
            terminal_padding_y: 4.0,
            perf_log_interval: 0.0,
            icons: IconConfig::default(),
            colors: ColorsConfig::default(),
            ui_colors: UiConfig::default(),
        }
    }
}

impl Config {
    /// Build a terminal theme from the color configuration.
    pub fn build_theme(&self) -> TerminalTheme {
        TerminalTheme::new(Box::new(self.colors.build_palette()))
    }

    /// Resolved terminal background as a Color32.
    pub fn terminal_background(&self) -> Color32 {
        let default_bg = ColorPalette::default().background;
        let hex = self.colors.background.as_deref().unwrap_or(&default_bg);
        hex_to_color32(hex).unwrap_or(Color32::from_rgb(0x18, 0x18, 0x18))
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
            terminal_padding_x = {terminal_padding_x},
            terminal_padding_y = {terminal_padding_y},
            perf_log_interval = {perf_log_interval},
            show_minimap = {show_minimap},
        }}
        "#,
        sidebar_width = sidebar_defaults.width,
        workspace_font_size = sidebar_defaults.workspace_font_size,
        terminal_title_font_size = sidebar_defaults.terminal_title_font_size,
        description_font_size = sidebar_defaults.description_font_size,
        terminal_font_size = config_defaults.terminal_font_size,
        terminal_padding_x = config_defaults.terminal_padding_x,
        terminal_padding_y = config_defaults.terminal_padding_y,
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

    // Parse colors config if present
    let colors = if let Ok(colors_table) = config_table.get::<mlua::Table>("colors") {
        ColorsConfig {
            foreground: colors_table.get("foreground").ok(),
            background: colors_table.get("background").ok(),
            black: colors_table.get("black").ok(),
            red: colors_table.get("red").ok(),
            green: colors_table.get("green").ok(),
            yellow: colors_table.get("yellow").ok(),
            blue: colors_table.get("blue").ok(),
            magenta: colors_table.get("magenta").ok(),
            cyan: colors_table.get("cyan").ok(),
            white: colors_table.get("white").ok(),
            bright_black: colors_table.get("bright_black").ok(),
            bright_red: colors_table.get("bright_red").ok(),
            bright_green: colors_table.get("bright_green").ok(),
            bright_yellow: colors_table.get("bright_yellow").ok(),
            bright_blue: colors_table.get("bright_blue").ok(),
            bright_magenta: colors_table.get("bright_magenta").ok(),
            bright_cyan: colors_table.get("bright_cyan").ok(),
            bright_white: colors_table.get("bright_white").ok(),
            dim_foreground: colors_table.get("dim_foreground").ok(),
            dim_black: colors_table.get("dim_black").ok(),
            dim_red: colors_table.get("dim_red").ok(),
            dim_green: colors_table.get("dim_green").ok(),
            dim_yellow: colors_table.get("dim_yellow").ok(),
            dim_blue: colors_table.get("dim_blue").ok(),
            dim_magenta: colors_table.get("dim_magenta").ok(),
            dim_cyan: colors_table.get("dim_cyan").ok(),
            dim_white: colors_table.get("dim_white").ok(),
        }
    } else {
        ColorsConfig::default()
    };

    // Parse UI colors config if present
    let ui_colors = if let Ok(ui_table) = config_table.get::<mlua::Table>("ui_colors") {
        let defaults = UiConfig::default();
        UiConfig {
            sidebar_background: ui_table
                .get::<String>("sidebar_background")
                .ok()
                .and_then(|s| hex_to_color32(&s))
                .unwrap_or(defaults.sidebar_background),
            sidebar_text: ui_table
                .get::<String>("sidebar_text")
                .ok()
                .and_then(|s| hex_to_color32(&s))
                .unwrap_or(defaults.sidebar_text),
            sidebar_text_dim: ui_table
                .get::<String>("sidebar_text_dim")
                .ok()
                .and_then(|s| hex_to_color32(&s))
                .unwrap_or(defaults.sidebar_text_dim),
            status_bar_background: ui_table
                .get::<String>("status_bar_background")
                .ok()
                .and_then(|s| hex_to_color32(&s))
                .unwrap_or(defaults.status_bar_background),
            status_bar_text: ui_table
                .get::<String>("status_bar_text")
                .ok()
                .and_then(|s| hex_to_color32(&s))
                .unwrap_or(defaults.status_bar_text),
            focused_border: ui_table
                .get::<String>("focused_border")
                .ok()
                .and_then(|s| hex_to_color32(&s))
                .unwrap_or(defaults.focused_border),
        }
    } else {
        UiConfig::default()
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
        terminal_padding_x: config_table.get("terminal_padding_x")?,
        terminal_padding_y: config_table.get("terminal_padding_y")?,
        perf_log_interval: config_table.get("perf_log_interval")?,
        icons,
        colors,
        ui_colors,
    };

    Ok(config)
}
