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
   - Position indicators (dots) at bottom of screen

2. **Terminal Simulation**
   - Character grid with cursor
   - Typing animation (lorem ipsum text)
   - Cursor blinking
   - Per-terminal text content

3. **Unix Socket IPC**
   - Control socket for external tooling
   - Ping command for liveness checking
   - Stale socket detection and cleanup
   - Duplicate instance prevention

### Architecture

```
src/
├── main.rs   # CLI entry point (clap-based)
├── gui.rs    # SDL2 GUI, WindowManager, Terminal structs
└── ipc.rs    # Unix socket server/client, protocol types
```

### Key Structures

**WindowManager** (`src/gui.rs`)
- Manages collection of terminals
- Handles scroll state (offset, target, animation)
- Tracks focused terminal index
- Calculates terminal positions from cumulative widths

**Terminal** (`src/gui.rs`)
- Character grid (cols × rows)
- Cursor position and blink state
- Width ratio (fraction of viewport)
- Text source for typing animation

**IpcServer/IpcClient** (`src/ipc.rs`)
- JSON protocol over Unix domain socket
- Non-blocking polling in main loop
- Request/Response types with serde

### Controls

| Key | Action |
|-----|--------|
| `Ctrl+N` | Create new terminal |
| `Ctrl+H` | Focus previous terminal |
| `Ctrl+L` | Focus next terminal |
| `Ctrl+,` | Shrink focused terminal |
| `Ctrl+.` | Grow focused terminal |
| `Escape` | Quit |

### CLI Usage

```bash
# Run without IPC
manse run

# Run with IPC socket
manse run --socket /tmp/manse.sock

# Ping a running instance
manse ping --socket /tmp/manse.sock
```

### Dependencies

- `sdl2` - Rendering and input (with ttf feature)
- `clap` - CLI argument parsing
- `serde` / `serde_json` - IPC protocol serialization

### Building

```bash
# macOS (requires SDL2 from homebrew)
LIBRARY_PATH="$(brew --prefix)/lib" cargo build

# Run
LIBRARY_PATH="$(brew --prefix)/lib" cargo run -- run
```

## Future Directions

### Planned: Real Terminal Emulation

The current implementation uses simulated terminals with lorem ipsum text. The next step is integrating actual terminal emulation using **libghostty** from the Ghostty project.

libghostty would provide:
- VT/ANSI escape sequence parsing
- PTY (pseudo-terminal) management
- Terminal state machine

Manse would continue to provide:
- Window management (scrolling, tiling)
- Rendering (via SDL2)
- IPC for external control

### Planned: Extended IPC

The socket interface is designed to support additional commands:

```json
// Get application state
{"cmd": "snapshot"}
{"ok": true, "result": {"terminals": [...], "focused": 0}}

// Send keypress
{"cmd": "send_key", "key": "ctrl+n"}
{"ok": true}
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
