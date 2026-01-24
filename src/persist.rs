//! Session persistence for suspend-and-restart.
//!
//! This module handles serializing application state to allow restarting
//! while preserving terminal sessions. PTY file descriptors survive across
//! exec() when CLOEXEC is cleared.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

/// Version for detecting incompatible state format changes.
/// Increment this when the serialization format changes.
pub const STATE_VERSION: u32 = 2;

/// Error type for persistence operations.
#[derive(Debug)]
pub enum PersistError {
    Io(io::Error),
    Json(serde_json::Error),
    VersionMismatch { expected: u32, found: u32 },
    InvalidFd(i32),
    ProcessNotRunning(u32),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "IO error: {}", e),
            PersistError::Json(e) => write!(f, "JSON error: {}", e),
            PersistError::VersionMismatch { expected, found } => {
                write!(f, "State version mismatch: expected {}, found {}", expected, found)
            }
            PersistError::InvalidFd(fd) => write!(f, "Invalid file descriptor: {}", fd),
            PersistError::ProcessNotRunning(pid) => write!(f, "Process not running: {}", pid),
        }
    }
}

impl std::error::Error for PersistError {}

impl From<io::Error> for PersistError {
    fn from(e: io::Error) -> Self {
        PersistError::Io(e)
    }
}

impl From<serde_json::Error> for PersistError {
    fn from(e: serde_json::Error) -> Self {
        PersistError::Json(e)
    }
}

/// Persisted application state.
#[derive(Serialize, Deserialize)]
pub struct PersistedState {
    /// Format version for compatibility checking.
    pub version: u32,
    /// All workspaces.
    pub workspaces: Vec<PersistedWorkspace>,
    /// Index of the active workspace.
    pub active_workspace: usize,
    /// Next internal panel ID to use.
    pub next_id: u64,
}

impl PersistedState {
    /// Save state to a file.
    pub fn save(&self, path: &Path) -> Result<(), PersistError> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }

    /// Load state from a file.
    pub fn load(path: &Path) -> Result<Self, PersistError> {
        let mut file = fs::File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let state: Self = serde_json::from_str(&contents)?;

        if state.version != STATE_VERSION {
            return Err(PersistError::VersionMismatch {
                expected: STATE_VERSION,
                found: state.version,
            });
        }

        Ok(state)
    }

    /// Validate that all file descriptors are still valid.
    #[cfg(unix)]
    pub fn validate_fds(&self) -> Vec<(usize, usize, PersistError)> {
        let mut errors = Vec::new();

        for (ws_idx, ws) in self.workspaces.iter().enumerate() {
            for (term_idx, term) in ws.terminals.iter().enumerate() {
                // Check if fd is valid
                let fd_valid = unsafe {
                    libc::fcntl(term.pty_fd, libc::F_GETFD) != -1
                };
                if !fd_valid {
                    errors.push((ws_idx, term_idx, PersistError::InvalidFd(term.pty_fd)));
                    continue;
                }

                // Check if process is still running
                let proc_running = unsafe {
                    libc::kill(term.pty_pid as i32, 0) == 0
                };
                if !proc_running {
                    errors.push((ws_idx, term_idx, PersistError::ProcessNotRunning(term.pty_pid)));
                }
            }
        }

        errors
    }
}

/// Persisted workspace state.
#[derive(Serialize, Deserialize)]
pub struct PersistedWorkspace {
    /// Workspace name.
    pub name: String,
    /// Order of panel internal IDs (left to right).
    pub panel_order: Vec<u64>,
    /// Index of the focused panel within this workspace.
    pub focused_index: usize,
    /// Terminals in this workspace.
    pub terminals: Vec<PersistedTerminal>,
}

/// Persisted terminal state.
#[derive(Serialize, Deserialize)]
pub struct PersistedTerminal {
    /// Internal ID (u64 used for HashMap key).
    pub internal_id: u64,
    /// External ID (the nanoid "term-xxx" string).
    pub external_id: String,
    /// PTY master file descriptor.
    pub pty_fd: i32,
    /// Child process ID.
    pub pty_pid: u32,
    /// Width ratio (fraction of viewport).
    pub width_ratio: f32,
    /// Custom title set via IPC.
    pub custom_title: Option<String>,
    /// Description text (set via in-app dialog).
    pub description: String,
    /// CLI description (set via manse term-desc).
    pub cli_description: Option<String>,
    /// Emoji icon.
    pub emoji: Option<String>,
    /// Current working directory (from OSC 7).
    pub cwd: Option<std::path::PathBuf>,
}

/// Clear the CLOEXEC flag on a file descriptor so it survives exec().
#[cfg(unix)]
pub fn clear_cloexec(fd: i32) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }

    let new_flags = flags & !libc::FD_CLOEXEC;
    let result = unsafe { libc::fcntl(fd, libc::F_SETFD, new_flags) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

/// Send SIGWINCH to a process to trigger terminal redraw.
#[cfg(unix)]
pub fn send_sigwinch(pid: u32) -> io::Result<()> {
    let result = unsafe { libc::kill(pid as i32, libc::SIGWINCH) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Get the current PTY window size.
#[cfg(unix)]
pub fn get_pty_size(fd: i32) -> io::Result<(u16, u16)> {
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut winsize) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok((winsize.ws_col, winsize.ws_row))
}

/// Set the PTY window size.
#[cfg(unix)]
pub fn set_pty_size(fd: i32, cols: u16, rows: u16) -> io::Result<()> {
    let winsize = libc::winsize {
        ws_col: cols,
        ws_row: rows,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &winsize) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Force a terminal redraw by toggling the PTY size.
/// This tricks applications into thinking the window resized, forcing a redraw.
#[cfg(unix)]
pub fn force_redraw(fd: i32, pid: u32) -> io::Result<()> {
    // Get current size
    let (cols, rows) = get_pty_size(fd)?;

    // Resize to slightly smaller
    let new_cols = if cols > 1 { cols - 1 } else { cols + 1 };
    set_pty_size(fd, new_cols, rows)?;

    // Send SIGWINCH for the smaller size
    send_sigwinch(pid)?;

    // Small delay to let the app process the resize
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Resize back to original
    set_pty_size(fd, cols, rows)?;

    // Send SIGWINCH for the restored size
    send_sigwinch(pid)?;

    Ok(())
}
