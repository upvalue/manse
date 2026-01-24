# Vendored Library Patches

Manse includes vendored/patched versions of several libraries. This document describes what changes have been made and why.

## alacritty_terminal

**Location:** `patches/alacritty_terminal/`

### PTY Session Restore Support

To support suspend-and-restart (preserving terminal sessions across binary restarts), we needed to:

1. Extract PTY file descriptors from running terminals
2. Restore terminals from existing file descriptors without spawning new shells

#### Changes to `src/tty/unix.rs`

**Struct changes:**
```rust
// Before:
pub struct Pty {
    child: Child,
    file: File,
    signals: UnixStream,
    sig_id: SigId,
}

// After:
pub struct Pty {
    child: Option<Child>,      // None when restored from fd
    child_pid: u32,            // Always available
    file: File,
    signals: Option<UnixStream>,  // None after into_raw_parts()
    sig_id: Option<SigId>,        // None after into_raw_parts()
}
```

**New methods on `Pty`:**
- `raw_fd() -> i32` - Get the PTY master file descriptor
- `child_pid() -> u32` - Get the child process ID
- `into_raw_parts() -> (i32, u32)` - Consume Pty without killing child, returns (fd, pid)

**New function:**
- `from_raw_fd(fd: i32, child_pid: u32) -> Result<Pty>` - Restore a Pty from existing fd

**Updated `Drop` impl:**
- Now handles `Option` fields gracefully
- Only kills child if `child` is `Some`

**Updated `EventedPty::next_child_event`:**
- For restored PTYs (no `Child` handle), uses `waitpid()` directly instead of `child.try_wait()`

**Updated `EventedReadWrite` impl:**
- `register`, `reregister`, `deregister` now handle optional `signals` field

#### Changes to `src/tty/mod.rs`

**Exports:**
```rust
pub use self::unix::{
    from_fd, from_raw_fd, new, Pty, ToWinsize,
    PTY_CHILD_EVENT_TOKEN, PTY_READ_WRITE_TOKEN,  // Now public
};
```

---

## egui_term

**Location:** `egui_term/`

This is a fork of [Harzu/egui_term](https://github.com/Harzu/egui_term).

### Focus Fix (Pre-existing)

**File:** `src/view.rs`

The upstream library requires both focus AND mouse hover for keyboard input. Our fork removes the hover requirement so terminals work properly when the window regains focus.

```rust
// Before (upstream):
if !layout.has_focus() || !layout.contains_pointer() {

// After (our fork):
if !layout.has_focus() {
```

### PTY Session Restore Support

**File:** `src/backend/mod.rs`

**Struct changes:**
```rust
pub struct TerminalBackend {
    id: u64,
    pty_id: u32,
    pty_fd: i32,  // NEW: stored for persistence
    // ... rest unchanged
}
```

**New constructor:**
```rust
/// Restore a terminal backend from an existing PTY file descriptor.
/// Used for session restore after exec.
#[cfg(not(windows))]
pub unsafe fn from_raw_fd(
    id: u64,
    pty_fd: i32,
    pty_id: u32,
    app_context: egui::Context,
    pty_event_proxy_sender: Sender<(u64, PtyEvent)>,
) -> Result<Self>
```

**New accessor:**
```rust
/// Get the PTY master file descriptor.
#[cfg(not(windows))]
pub fn pty_fd(&self) -> i32
```

---

## vte

**Location:** `patches/vte/`

This is a vendored copy of vte 0.15.0 (VT parser library used by alacritty_terminal).

**Status:** No code changes from upstream. Vendored for version pinning to ensure compatibility with our alacritty_terminal patches.

---

## Why Vendor?

These patches are tightly coupled to Manse's suspend-and-restart feature. Upstreaming would be difficult because:

1. **alacritty_terminal** - The changes make `Pty` more complex to support a niche use case (session restore). The alacritty project likely wouldn't want this complexity.

2. **egui_term** - The focus fix might be upstreamable, but the session restore support depends on our alacritty_terminal patches.

For now, we maintain these as local patches. If the upstream projects become interested, we could propose the changes.
