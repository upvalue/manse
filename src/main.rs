mod app;
mod config;
mod fonts;
mod ipc_protocol;
mod persist;
mod terminal;
mod ui;
mod util;
mod workspace;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "manse")]
#[command(about = "A scrolling window manager for terminals")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the terminal window manager
    Run {
        /// Path to IPC socket
        #[arg(short, long, default_value = "/tmp/manse.sock")]
        socket: PathBuf,
    },
    /// Resume from persisted state (internal, called after exec)
    Resume {
        /// Path to state file
        #[arg(long)]
        state_file: PathBuf,
        /// Path to IPC socket
        #[arg(short, long, default_value = "/tmp/manse.sock")]
        socket: PathBuf,
    },
    /// Trigger restart of running instance
    Restart {
        /// Path to IPC socket
        #[arg(short, long, default_value = "/tmp/manse.sock")]
        socket: PathBuf,
    },
    /// Ping a running instance
    Ping {
        /// Path to IPC socket
        #[arg(short, long, default_value = "/tmp/manse.sock")]
        socket: PathBuf,
    },
    /// Rename a terminal
    TermRename {
        /// Path to IPC socket (defaults to $MANSE_SOCKET or /tmp/manse.sock)
        #[arg(short, long, env = "MANSE_SOCKET", default_value = "/tmp/manse.sock")]
        socket: PathBuf,
        /// Terminal ID (defaults to $MANSE_TERMINAL)
        #[arg(short, long, env = "MANSE_TERMINAL")]
        terminal: String,
        /// New title for the terminal
        title: String,
    },
    /// Set terminal description
    TermDesc {
        /// Path to IPC socket (defaults to $MANSE_SOCKET or /tmp/manse.sock)
        #[arg(short, long, env = "MANSE_SOCKET", default_value = "/tmp/manse.sock")]
        socket: PathBuf,
        /// Terminal ID (defaults to $MANSE_TERMINAL)
        #[arg(short, long, env = "MANSE_TERMINAL")]
        terminal: String,
        /// Description for the terminal
        description: String,
    },
    /// Set terminal icon (Nerd Font codepoint)
    TermIcon {
        /// Path to IPC socket (defaults to $MANSE_SOCKET or /tmp/manse.sock)
        #[arg(short, long, env = "MANSE_SOCKET", default_value = "/tmp/manse.sock")]
        socket: PathBuf,
        /// Terminal ID (defaults to $MANSE_TERMINAL)
        #[arg(short, long, env = "MANSE_TERMINAL")]
        terminal: String,
        /// Icon for the terminal (Nerd Font codepoint, empty string to clear)
        icon: String,
    },
    /// Move a terminal to a workspace (creates workspace if needed)
    TermToWorkspace {
        /// Path to IPC socket (defaults to $MANSE_SOCKET or /tmp/manse.sock)
        #[arg(short, long, env = "MANSE_SOCKET", default_value = "/tmp/manse.sock")]
        socket: PathBuf,
        /// Terminal ID (defaults to $MANSE_TERMINAL)
        #[arg(short, long, env = "MANSE_TERMINAL")]
        terminal: String,
        /// Name of the workspace to move to
        #[arg(short, long)]
        workspace_name: String,
    },
    /// Notify a terminal (shows indicator until focused)
    TermNotify {
        /// Path to IPC socket (defaults to $MANSE_SOCKET or /tmp/manse.sock)
        #[arg(short, long, env = "MANSE_SOCKET", default_value = "/tmp/manse.sock")]
        socket: PathBuf,
        /// Terminal ID (defaults to $MANSE_TERMINAL)
        #[arg(short, long, env = "MANSE_TERMINAL")]
        terminal: String,
    },
    /// Initialize a .manse.json project file in the current directory
    Init {
        /// Project name (defaults to current directory name)
        name: Option<String>,
    },
}

/// Run a fresh instance (no restore).
fn run_fresh(socket: PathBuf, config: config::Config) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([400.0, 300.0])
            .with_maximized(true),
        ..Default::default()
    };

    eframe::run_native(
        "manse",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, Some(socket), config)))),
    )
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { socket } => {
            let config = config::load_config();

            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([1200.0, 800.0])
                    .with_min_inner_size([400.0, 300.0])
                    .with_maximized(true),
                ..Default::default()
            };

            eframe::run_native(
                "manse",
                options,
                Box::new(move |cc| Ok(Box::new(app::App::new(cc, Some(socket), config)))),
            )
        }
        Commands::Resume { state_file, socket } => {
            let config = config::load_config();

            // Load persisted state
            let state = match persist::PersistedState::load(&state_file) {
                Ok(state) => state,
                Err(e) => {
                    log::warn!("Failed to load persisted state: {}. Starting fresh.", e);
                    // Clean up the state file
                    let _ = std::fs::remove_file(&state_file);
                    // Fall back to fresh start
                    return run_fresh(socket, config);
                }
            };

            // Validate file descriptors
            let errors = state.validate_fds();
            if !errors.is_empty() {
                for (ws_idx, term_idx, err) in &errors {
                    log::warn!(
                        "Terminal {} in workspace {} failed validation: {}",
                        term_idx, ws_idx, err
                    );
                }
            }

            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([1200.0, 800.0])
                    .with_min_inner_size([400.0, 300.0])
                    .with_maximized(true),
                ..Default::default()
            };

            // Clean up state file after loading
            let _ = std::fs::remove_file(&state_file);

            eframe::run_native(
                "manse",
                options,
                Box::new(move |cc| {
                    match app::App::from_persisted(cc, state, socket.clone(), config.clone()) {
                        Ok(app) => Ok(Box::new(app)),
                        Err(e) => {
                            log::warn!("Failed to restore from persisted state: {}. Starting fresh.", e);
                            Ok(Box::new(app::App::new(cc, Some(socket), config)))
                        }
                    }
                }),
            )
        }
        Commands::Restart { socket } => {
            let mut client = ipc_protocol::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc_protocol::Request::Restart)
                .map_err(|e| eprintln!("Request failed: {}", e))
                .unwrap();

            if response.ok {
                println!("Restart initiated");
            } else {
                eprintln!(
                    "Failed to restart: {}",
                    response.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
            Ok(())
        }
        Commands::Ping { socket } => {
            let mut client = ipc_protocol::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            match client.ping() {
                Ok(()) => {
                    println!("Pong!");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Ping failed: {}", e);
                    Ok(())
                }
            }
        }
        Commands::TermRename {
            socket,
            terminal,
            title,
        } => {
            let mut client = ipc_protocol::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc_protocol::Request::TermRename { terminal, title })
                .map_err(|e| eprintln!("Request failed: {}", e))
                .unwrap();

            if response.ok {
                println!("Terminal renamed");
            } else {
                eprintln!(
                    "Failed to rename: {}",
                    response.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
            Ok(())
        }
        Commands::TermDesc {
            socket,
            terminal,
            description,
        } => {
            let mut client = ipc_protocol::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc_protocol::Request::TermDesc { terminal, description })
                .map_err(|e| eprintln!("Request failed: {}", e))
                .unwrap();

            if response.ok {
                println!("Terminal description set");
            } else {
                eprintln!(
                    "Failed to set description: {}",
                    response.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
            Ok(())
        }
        Commands::TermIcon {
            socket,
            terminal,
            icon,
        } => {
            let mut client = ipc_protocol::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc_protocol::Request::TermIcon { terminal, icon })
                .map_err(|e| eprintln!("Request failed: {}", e))
                .unwrap();

            if response.ok {
                println!("Terminal icon set");
            } else {
                eprintln!(
                    "Failed to set icon: {}",
                    response.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
            Ok(())
        }
        Commands::TermToWorkspace {
            socket,
            terminal,
            workspace_name,
        } => {
            let mut client = ipc_protocol::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc_protocol::Request::TermToWorkspace {
                    terminal,
                    workspace_name: workspace_name.clone(),
                })
                .map_err(|e| eprintln!("Request failed: {}", e))
                .unwrap();

            if response.ok {
                println!("Terminal moved to workspace '{}'", workspace_name);
            } else {
                eprintln!(
                    "Failed to move terminal: {}",
                    response.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
            Ok(())
        }
        Commands::TermNotify { socket, terminal } => {
            let mut client = ipc_protocol::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc_protocol::Request::TermNotify { terminal })
                .map_err(|e| eprintln!("Request failed: {}", e))
                .unwrap();

            if response.ok {
                println!("Terminal notified");
            } else {
                eprintln!(
                    "Failed to notify: {}",
                    response.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
            Ok(())
        }
        Commands::Init { name } => {
            let project_name = name.unwrap_or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "project".to_string())
            });

            let config = serde_json::json!({
                "workspaceName": project_name
            });

            let path = PathBuf::from(".manse.json");
            if path.exists() {
                eprintln!(".manse.json already exists");
                return Ok(());
            }

            match std::fs::write(&path, serde_json::to_string_pretty(&config).unwrap()) {
                Ok(()) => println!("Created .manse.json with name: {}", project_name),
                Err(e) => eprintln!("Failed to create .manse.json: {}", e),
            }
            Ok(())
        }
    }
}
