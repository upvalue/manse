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

3. **Unix Socket IPC**
   - Control socket for external tooling
   - Ping command for liveness checking
   - Stale socket detection and cleanup
   - Duplicate instance prevention

4. **UI Layout**
   - Left sidebar (200px, placeholder for future features)
   - Status bar with terminal minimap and position indicator
   - Blue border highlight on focused terminal

### Architecture

```
manse-rs/
├── src/
│   ├── main.rs   # CLI entry point (clap + eframe)
│   ├── app.rs    # egui App, WindowManager logic, terminal panels
│   └── ipc.rs    # Unix socket server/client, protocol types
└── egui_term/    # Local fork of egui_term (focus fix applied)
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
- Non-blocking polling in main loop
- Request/Response types with serde

### Controls

| Key | Action |
|-----|--------|
| `Ctrl+N` | Create new terminal |
| `Ctrl+W` | Close focused terminal |
| `Ctrl+H` | Focus previous terminal |
| `Ctrl+L` | Focus next terminal |
| `Ctrl+,` | Shrink focused terminal |
| `Ctrl+.` | Grow focused terminal |

### CLI Usage

```bash
# Run without IPC
cargo run -- run

# Run with IPC socket
cargo run -- run --socket /tmp/manse.sock

# Ping a running instance
cargo run -- ping --socket /tmp/manse.sock
```

### Dependencies

- `eframe` / `egui` - GUI framework
- `egui_term` - Terminal widget (local fork with focus fix)
- `alacritty_terminal` - Terminal emulation backend (via egui_term)
- `clap` - CLI argument parsing
- `serde` / `serde_json` - IPC protocol serialization

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

## Future Directions

### Planned: Extended IPC

The socket interface is designed to support additional commands:

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
