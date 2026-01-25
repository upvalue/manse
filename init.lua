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


-- presentation settings
config.sidebar_width = 300
config.workspace_font_size = 16
config.terminal_title_font_size = 16
config.description_font_size = 16
config.terminal_font_size = 18
