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
    /// Description set via in-app dialog (Cmd+D)
    pub description: String,
    /// Description set via CLI/IPC (manse term-desc)
    pub cli_description: Option<String>,
    /// Optional icon (Nerd Font codepoint) set via IPC
    pub icon: Option<String>,
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
        let term_id = crate::util::ids::new_terminal_id();

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        // Set environment variables for the terminal
        let mut env = HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
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
            cli_description: None,
            icon: None,
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
            title: if persisted.title.is_empty() {
                String::from("Terminal")
            } else {
                persisted.title.clone()
            },
            custom_title: persisted.custom_title.clone(),
            description: persisted.description.clone(),
            cli_description: persisted.cli_description.clone(),
            icon: persisted.icon.clone(),
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
            title: self.title.clone(),
            custom_title: self.custom_title.clone(),
            description: self.description.clone(),
            cli_description: self.cli_description.clone(),
            icon: self.icon.clone(),
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

    /// Check if this terminal is running an SSH session by inspecting the process tree.
    /// Returns the parsed SSH info if found.
    pub fn detect_ssh(&self) -> Option<SshSession> {
        detect_ssh_in_process_tree(self.pty_pid())
    }
}

/// Information about a detected SSH session.
#[derive(Debug, Clone)]
pub struct SshSession {
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub full_command: String,
}

impl std::fmt::Display for SshSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(user) = &self.user {
            write!(f, "{}@{}", user, self.host)?;
        } else {
            write!(f, "{}", self.host)?;
        }
        if let Some(port) = self.port {
            write!(f, ":{}", port)?;
        }
        Ok(())
    }
}

/// Walk the process tree rooted at `pid` looking for an ssh process.
/// Uses `ps` to find descendant processes.
fn detect_ssh_in_process_tree(pid: u32) -> Option<SshSession> {
    // Get all processes with their parent pid and command
    let output = std::process::Command::new("ps")
        .args(["-eo", "pid,ppid,comm,args"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);

    // Build a map of pid -> (ppid, command, full_args)
    let mut children: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    let mut commands: std::collections::HashMap<u32, (String, String)> =
        std::collections::HashMap::new();

    for line in text.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let Some(p) = parts[0].parse::<u32>().ok() else {
            continue;
        };
        let Some(ppid) = parts[1].parse::<u32>().ok() else {
            continue;
        };
        let comm = parts[2].to_string();
        let args = parts[3..].join(" ");
        children.entry(ppid).or_default().push(p);
        commands.insert(p, (comm, args));
    }

    // BFS from pid to find any descendant running ssh
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(pid);

    while let Some(current) = queue.pop_front() {
        if let Some(kids) = children.get(&current) {
            for &kid in kids {
                if let Some((comm, args)) = commands.get(&kid) {
                    if comm == "ssh" || comm.ends_with("/ssh") {
                        return parse_ssh_args(args);
                    }
                }
                queue.push_back(kid);
            }
        }
    }

    None
}

/// Parse an ssh command line to extract user, host, and port.
fn parse_ssh_args(args: &str) -> Option<SshSession> {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let full_command = args.to_string();
    let mut port: Option<u16> = None;
    let mut destination: Option<&str> = None;

    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if part == "ssh" || part.ends_with("/ssh") {
            // skip the binary itself
            i += 1;
            continue;
        }
        if part == "-p" {
            // next arg is port
            if i + 1 < parts.len() {
                port = parts[i + 1].parse().ok();
                i += 2;
                continue;
            }
        }
        // Skip flags that take an argument
        if part.starts_with('-')
            && part.len() == 2
            && matches!(
                part.chars().nth(1),
                Some('b' | 'c' | 'D' | 'E' | 'e' | 'F' | 'I' | 'i' | 'J' | 'L' | 'l'
                    | 'm' | 'O' | 'o' | 'Q' | 'R' | 'S' | 'W' | 'w')
            )
        {
            i += 2; // skip flag + its argument
            continue;
        }
        // Skip boolean flags
        if part.starts_with('-') {
            i += 1;
            continue;
        }
        // First non-flag argument is the destination
        if destination.is_none() {
            destination = Some(part);
        }
        // Anything after destination is the remote command — stop
        break;
    }

    let dest = destination?;

    // Parse user@host or just host
    let (user, host) = if let Some(at_pos) = dest.find('@') {
        (
            Some(dest[..at_pos].to_string()),
            dest[at_pos + 1..].to_string(),
        )
    } else {
        (None, dest.to_string())
    };

    Some(SshSession {
        user,
        host,
        port,
        full_command,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_host() {
        let s = parse_ssh_args("ssh mybox").unwrap();
        assert_eq!(s.host, "mybox");
        assert!(s.user.is_none());
        assert!(s.port.is_none());
    }

    #[test]
    fn parse_user_at_host() {
        let s = parse_ssh_args("ssh alice@prod-server.example.com").unwrap();
        assert_eq!(s.user.as_deref(), Some("alice"));
        assert_eq!(s.host, "prod-server.example.com");
        assert!(s.port.is_none());
    }

    #[test]
    fn parse_with_port() {
        let s = parse_ssh_args("ssh -p 2222 root@box").unwrap();
        assert_eq!(s.user.as_deref(), Some("root"));
        assert_eq!(s.host, "box");
        assert_eq!(s.port, Some(2222));
    }

    #[test]
    fn parse_with_identity_file() {
        let s = parse_ssh_args("ssh -i ~/.ssh/id_ed25519 deploy@staging").unwrap();
        assert_eq!(s.user.as_deref(), Some("deploy"));
        assert_eq!(s.host, "staging");
    }

    #[test]
    fn parse_with_mixed_flags() {
        let s = parse_ssh_args("ssh -A -o StrictHostKeyChecking=no -p 22 me@host").unwrap();
        assert_eq!(s.user.as_deref(), Some("me"));
        assert_eq!(s.host, "host");
        assert_eq!(s.port, Some(22));
    }

    #[test]
    fn parse_full_path_binary() {
        let s = parse_ssh_args("/usr/bin/ssh user@box").unwrap();
        assert_eq!(s.user.as_deref(), Some("user"));
        assert_eq!(s.host, "box");
    }

    #[test]
    fn display_user_host_port() {
        let s = SshSession {
            user: Some("alice".into()),
            host: "prod".into(),
            port: Some(2222),
            full_command: String::new(),
        };
        assert_eq!(s.to_string(), "alice@prod:2222");
    }

    #[test]
    fn display_host_only() {
        let s = SshSession {
            user: None,
            host: "mybox".into(),
            port: None,
            full_command: String::new(),
        };
        assert_eq!(s.to_string(), "mybox");
    }
}
