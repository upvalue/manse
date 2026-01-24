# Manse - Scrolling Window Manager for Terminals

## Overview

Manse is a prototype scrolling window manager for terminal emulators, inspired by PaperWM and Niri. Terminals are arranged in a horizontal strip and can have variable widths, allowing multiple terminals to be visible simultaneously.

## Current State

### Features Implemented

1. **Scrolling Window Manager**
   - Horizontal arrangement of terminal panels
   - Smooth animated scrolling between terminals
   - Variable-width terminals (1/3, 1/2, 2/3, or full viewport width)
   - Multiple terminals visible when they fit in viewport
   - Position indicators (dots) in status bar

2. **Real Terminal Emulation**
   - Full PTY-based terminal emulation via egui_term/alacritty_terminal
   - Spawns user's default shell ($SHELL)
   - Proper VT/ANSI escape sequence handling
   - OSC 7 support for current working directory tracking

3. **Unix Socket IPC**
   - Control socket for external tooling
   - Multithreaded IPC listener
   - Stale socket detection and cleanup
   - Duplicate instance prevention
   - Terminal management commands (rename, describe, move to workspace)

4. **Workspaces**
   - Organize terminals into named workspaces
   - Move terminals between workspaces via IPC
   - Workspace switching in sidebar

5. **UI Layout**
   - Left sidebar with workspace/terminal tree
   - Status bar with terminal minimap and position indicator
   - Blue border highlight on focused terminal
   - Command palette (⌘P) for quick actions

6. **Lua Configuration**
   - `init.lua` for customizing sidebar, fonts, etc.
   - Runtime configuration loading

7. **Shell/Editor Integration**
   - Fish shell plugin (`plugins/fish/`)
   - Neovim plugin (`plugins/neovim/`)
   - Environment variables for IPC (MANSE_SOCKET, MANSE_TERMINAL)

### Architecture

```
manse/
├── src/
│   ├── main.rs       # CLI entry point (clap + eframe)
│   ├── app.rs        # egui App, WindowManager logic, terminal panels
│   ├── command.rs    # Command definitions for palette
│   ├── config.rs     # Lua configuration loader
│   ├── icons.rs      # Icon loading (e.g., neovim icon)
│   ├── ipc.rs        # Unix socket server/client, protocol types
│   ├── terminal.rs   # Terminal panel abstraction
│   ├── workspace.rs  # Workspace data structure
│   └── ui/
│       ├── mod.rs
│       ├── command_palette.rs  # ⌘P command palette
│       ├── sidebar.rs          # Workspace/terminal sidebar
│       └── status_bar.rs       # Terminal position indicators
├── egui_term/        # Local fork of egui_term (focus fix applied)
├── patches/          # Patched dependencies
│   ├── alacritty_terminal/
│   └── vte/
├── plugins/          # Shell/editor integrations
│   ├── fish/         # Fish shell integration
│   └── neovim/       # Neovim plugin
└── init.lua          # User configuration (Lua)
```

### Key Structures

**App** (`src/app.rs`)
- Manages collection of TerminalPanel instances
- Handles scroll state (offset, target, animation)
- Tracks focused terminal index
- Processes IPC commands
- Renders UI (sidebar, status bar, terminal area)

**TerminalPanel** (`src/app.rs`)
- Wraps egui_term::TerminalBackend
- Width ratio (fraction of viewport)
- Unique ID for event routing

**IpcServer/IpcClient** (`src/ipc.rs`)
- JSON protocol over Unix domain socket
- Multithreaded listener with channel-based message passing
- Request/Response types with serde
- Commands: Ping, TermRename, TermDesc, TermToWorkspace

**Workspace** (`src/workspace.rs`)
- Named container for grouping terminals
- UUID-based terminal membership

### Controls

All keybindings use ⌘ (Cmd) to avoid conflicts with terminal applications.

| Key | Action |
|-----|--------|
| `⌘T` | Create new terminal |
| `⌘W` | Close focused terminal |
| `⌘[` | Focus previous terminal |
| `⌘]` | Focus next terminal |
| `⌘⇧[` | Swap with previous terminal |
| `⌘⇧]` | Swap with next terminal |
| `⌘-` | Shrink focused terminal |
| `⌘=` | Grow focused terminal |
| `⌘P` | Toggle command palette |

### CLI Usage

```bash
# Run the terminal manager (socket defaults to /tmp/manse.sock)
cargo run -- run
cargo run -- run --socket /tmp/manse.sock

# Ping a running instance
cargo run -- ping --socket /tmp/manse.sock

# Rename a terminal (uses $MANSE_SOCKET and $MANSE_TERMINAL env vars)
cargo run -- term-rename "My Terminal"
cargo run -- term-rename -t <uuid> "My Terminal"

# Set terminal description
cargo run -- term-desc "Working on feature X"

# Move terminal to workspace
cargo run -- term-to-workspace -w "project-a"

# Notify a terminal (shows indicator until focused)
cargo run -- term-notify
cargo run -- term-notify -t <uuid>
```

### Environment Variables

Terminals spawned by Manse have these environment variables set:
- `MANSE_SOCKET` - Path to the IPC socket
- `MANSE_TERMINAL` - UUID of the terminal

This enables shell scripts and editor plugins to communicate with Manse.

### Configuration

Manse loads configuration from `init.lua` in the project root:

```lua
-- init.lua example
config.sidebar_width = 300
config.workspace_font_size = 13
config.terminal_title_font_size = 12
config.description_font_size = 10
config.terminal_font_size = 14
```

### Dependencies

- `eframe` / `egui` - GUI framework
- `egui_term` - Terminal widget (local fork with focus fix)
- `alacritty_terminal` - Terminal emulation backend (patched in `patches/`)
- `vte` - VT parser (patched in `patches/`)
- `clap` - CLI argument parsing
- `serde` / `serde_json` - IPC protocol serialization
- `mlua` - Lua configuration scripting

### Building

```bash
cargo build
cargo run -- run
```

### Local egui_term Fork

The `egui_term/` directory contains a fork of [Harzu/egui_term](https://github.com/Harzu/egui_term) with a fix for keyboard focus handling. The upstream library requires both focus AND mouse hover for keyboard input; our fork removes the hover requirement so terminals work properly when the window regains focus.

Change in `egui_term/src/view.rs`:
```rust
// Before (upstream):
if !layout.has_focus() || !layout.contains_pointer() {

// After (our fork):
if !layout.has_focus() {
```

### IPC Protocol

The socket interface supports these commands:

```json
// Ping for liveness
{"cmd": "ping"}
{"ok": true}

// Rename a terminal
{"cmd": "term_rename", "terminal": "<uuid>", "title": "My Terminal"}
{"ok": true}

// Set terminal description
{"cmd": "term_desc", "terminal": "<uuid>", "description": "Working on X"}
{"ok": true}

// Move terminal to workspace
{"cmd": "term_to_workspace", "terminal": "<uuid>", "workspace_name": "project"}
{"ok": true}

// Notify a terminal (shows indicator until focused)
{"cmd": "term_notify", "terminal": "<uuid>"}
{"ok": true}
```

## Future Directions

### Planned: Extended IPC

Additional commands being considered:

```json
// Get application state
{"cmd": "snapshot"}
{"ok": true, "result": {"terminals": [...], "focused": 0}}

// Terminal management
{"cmd": "new_terminal"}
{"cmd": "close_terminal"}
{"cmd": "focus_next"}
{"cmd": "focus_prev"}
```

This enables:
- Integration testing
- Scripting and automation
- External tools inspecting/controlling state

### Design Philosophy

Inspired by scrolling window managers like PaperWM and Niri:
- Windows arranged in continuous horizontal strip
- Variable window widths
- Scroll to navigate (not discrete workspaces)
- Multiple windows visible when they fit
