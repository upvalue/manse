use eframe::egui;
use egui_term::{BackendSettings, PtyEvent, TerminalBackend};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use uuid::Uuid;

// howdypal!

/// A terminal panel in the window manager
pub struct TerminalPanel {
    /// Unique identifier for external reference
    pub uuid: Uuid,
    pub backend: TerminalBackend,
    pub width_ratio: f32,
    /// Terminal title (from shell escape sequences)
    pub title: String,
    /// Custom title set via IPC (overrides natural title when Some)
    pub custom_title: Option<String>,
    /// Optional description set via IPC
    pub description: String,
    /// Current working directory (from OSC 7 escape sequences)
    pub current_working_directory: Option<PathBuf>,
}

impl TerminalPanel {
    pub fn new(
        id: u64,
        ctx: &egui::Context,
        event_tx: Sender<(u64, PtyEvent)>,
        socket_path: Option<&PathBuf>,
        working_directory: Option<PathBuf>,
    ) -> Self {
        let uuid = Uuid::new_v4();

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        // Set environment variables for the terminal
        let mut env = HashMap::new();
        env.insert("MANSE_TERMINAL".to_string(), uuid.to_string());
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
            uuid,
            backend,
            width_ratio: 1.0,
            title: String::from("Terminal"),
            custom_title: None,
            description: String::new(),
            current_working_directory: working_directory,
        }
    }

    /// Returns the display title (custom title if set, otherwise natural title)
    pub fn display_title(&self) -> &str {
        self.custom_title.as_deref().unwrap_or(&self.title)
    }

    pub fn pixel_width(&self, viewport_width: f32) -> f32 {
        viewport_width * self.width_ratio
    }
}
