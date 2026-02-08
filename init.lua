-- Manse configuration
-- This file is loaded at startup from the project root

print('hi!')

-- Performance logging: set to > 0 to log frame/event stats every N seconds
-- Run with RUST_LOG=info to see output
config.perf_log_interval = 5

-- Icon aliases
local TERMINAL = "🖥️"
local ROBOT = "🤖"
local PENCIL = "✏️"

-- Icon detection patterns (checked in order, first match wins)
config.icons = {
  default = TERMINAL,
  patterns = {
    { match = "claude", icon = ROBOT },
    { match = "nvim", icon = PENCIL },
    { match = "neovim", icon = PENCIL },
  }
}

--[[
config.sidebar_width = 250
config.workspace_font_size = 14
config.terminal_title_font_size = 14
config.description_font_size = 14
config.terminal_font_size = 14
]]


-- Fonts
config.font_family = "Iosevka"  -- system font name (default: built-in JetBrains Mono)
config.terminal_font_size = 16
config.workspace_font_size = 16
config.terminal_title_font_size = 16
config.description_font_size = 16
-- config.status_bar_title_font_size = 12    -- default: 12
-- config.status_bar_description_font_size = 11  -- default: 11

-- Layout
config.sidebar_width = 300

-- Iceberg color scheme (from wezterm/iTerm2)
config.colors = {
  foreground = "#c6c8d1",
  background = "#161821",
  black = "#1e2132",
  red = "#e27878",
  green = "#b4be82",
  yellow = "#e2a478",
  blue = "#84a0c6",
  magenta = "#a093c7",
  cyan = "#89b8c2",
  white = "#c6c8d1",
  bright_black = "#6b7089",
  bright_red = "#e98989",
  bright_green = "#c0ca8e",
  bright_yellow = "#e9b189",
  bright_blue = "#91acd1",
  bright_magenta = "#ada0d3",
  bright_cyan = "#95c4ce",
  bright_white = "#d2d4de",
}

-- UI theme (sidebar, status bar, borders)
config.ui_colors = {
  sidebar_background = "#1e2132",
  sidebar_text = "#c6c8d1",
  sidebar_text_dim = "#6b7089",
  status_bar_background = "#161821",
  status_bar_text = "#6b7089",
  focused_border = "#84a0c6",
}
