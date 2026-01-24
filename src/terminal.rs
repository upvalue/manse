use crate::persist::PersistedTerminal;
use eframe::egui;
use egui_term::{BackendSettings, PtyEvent, TerminalBackend};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

// howdypal!

/// A terminal panel in the window manager
pub struct TerminalPanel {
    /// Unique identifier for external reference (nanoid with "term-" prefix)
    pub id: String,
    pub backend: TerminalBackend,
    pub width_ratio: f32,
    /// Terminal title (from shell escape sequences)
    pub title: String,
    /// Custom title set via IPC (overrides natural title when Some)
    pub custom_title: Option<String>,
    /// Optional description set via IPC
    pub description: String,
    /// Optional emoji icon set via IPC
    pub emoji: Option<String>,
    /// Current working directory (from OSC 7 escape sequences)
    pub current_working_directory: Option<PathBuf>,
    /// Whether this terminal has a pending notification
    pub notified: bool,
}

impl TerminalPanel {
    pub fn new(
        id: u64,
        ctx: &egui::Context,
        event_tx: Sender<(u64, PtyEvent)>,
        socket_path: Option<&PathBuf>,
        working_directory: Option<PathBuf>,
    ) -> Self {
        let term_id = crate::id::new_terminal_id();

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        // Set environment variables for the terminal
        let mut env = HashMap::new();
        env.insert("MANSE_TERMINAL".to_string(), term_id.clone());
        if let Some(path) = socket_path {
            env.insert("MANSE_SOCKET".to_string(), path.display().to_string());
        }

        // Use provided working directory or fall back to current dir
        let working_directory = working_directory.or_else(|| std::env::current_dir().ok());

        let settings = BackendSettings {
            shell,
            working_directory: working_directory.clone(),
            env,
            ..Default::default()
        };

        let backend = TerminalBackend::new(id, ctx.clone(), event_tx, settings)
            .expect("Failed to create terminal backend");

        Self {
            id: term_id,
            backend,
            width_ratio: 1.0,
            title: String::from("Terminal"),
            custom_title: None,
            description: String::new(),
            emoji: None,
            current_working_directory: working_directory,
            notified: false,
        }
    }

    /// Returns the display title (custom title if set, otherwise natural title)
    pub fn display_title(&self) -> &str {
        self.custom_title.as_deref().unwrap_or(&self.title)
    }

    pub fn pixel_width(&self, viewport_width: f32) -> f32 {
        viewport_width * self.width_ratio
    }

    /// Restore a terminal panel from persisted state.
    ///
    /// # Safety
    /// The PTY fd must be valid and the process must be running.
    #[cfg(unix)]
    pub unsafe fn from_persisted(
        internal_id: u64,
        persisted: &PersistedTerminal,
        ctx: &egui::Context,
        event_tx: Sender<(u64, PtyEvent)>,
    ) -> io::Result<Self> {
        let backend = unsafe {
            TerminalBackend::from_raw_fd(
                internal_id,
                persisted.pty_fd,
                persisted.pty_pid,
                ctx.clone(),
                event_tx,
            )?
        };

        Ok(Self {
            id: persisted.external_id.clone(),
            backend,
            width_ratio: persisted.width_ratio,
            title: String::from("Terminal"),
            custom_title: persisted.custom_title.clone(),
            description: persisted.description.clone(),
            emoji: persisted.emoji.clone(),
            current_working_directory: persisted.cwd.clone(),
            notified: false,
        })
    }

    /// Convert to persisted form for serialization.
    #[cfg(unix)]
    pub fn to_persisted(&self, internal_id: u64) -> PersistedTerminal {
        PersistedTerminal {
            internal_id,
            external_id: self.id.clone(),
            pty_fd: self.backend.pty_fd(),
            pty_pid: self.backend.pty_id(),
            width_ratio: self.width_ratio,
            custom_title: self.custom_title.clone(),
            description: self.description.clone(),
            emoji: self.emoji.clone(),
            cwd: self.current_working_directory.clone(),
        }
    }

    /// Get the PTY file descriptor.
    #[cfg(unix)]
    pub fn pty_fd(&self) -> i32 {
        self.backend.pty_fd()
    }

    /// Get the PTY child process ID.
    pub fn pty_pid(&self) -> u32 {
        self.backend.pty_id()
    }
}
