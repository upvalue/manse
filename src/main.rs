mod app;
mod command;
mod config;
mod ipc;
mod terminal;
mod ui;
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
        /// Terminal UUID (defaults to $MANSE_TERMINAL)
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
        /// Terminal UUID (defaults to $MANSE_TERMINAL)
        #[arg(short, long, env = "MANSE_TERMINAL")]
        terminal: String,
        /// Description for the terminal
        description: String,
    },
    /// Move a terminal to a workspace (creates workspace if needed)
    TermToWorkspace {
        /// Path to IPC socket (defaults to $MANSE_SOCKET or /tmp/manse.sock)
        #[arg(short, long, env = "MANSE_SOCKET", default_value = "/tmp/manse.sock")]
        socket: PathBuf,
        /// Terminal UUID (defaults to $MANSE_TERMINAL)
        #[arg(short, long, env = "MANSE_TERMINAL")]
        terminal: String,
        /// Name of the workspace to move to
        #[arg(short, long)]
        workspace_name: String,
    },
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
        Commands::Ping { socket } => {
            let mut client = ipc::IpcClient::connect(&socket)
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
            let mut client = ipc::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc::Request::TermRename { terminal, title })
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
            let mut client = ipc::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc::Request::TermDesc { terminal, description })
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
        Commands::TermToWorkspace {
            socket,
            terminal,
            workspace_name,
        } => {
            let mut client = ipc::IpcClient::connect(&socket)
                .map_err(|e| eprintln!("Failed to connect: {}", e))
                .unwrap();

            let response = client
                .request(&ipc::Request::TermToWorkspace {
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
    }
}
