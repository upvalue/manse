mod gui;
mod ipc;
mod terminal;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "manse")]
#[command(about = "A scrolling window manager for terminals", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the window manager GUI
    Run {
        /// Path to the Unix socket for IPC
        #[arg(long)]
        socket: Option<PathBuf>,
    },
    /// Ping a running instance to check if it's alive
    Ping {
        /// Path to the Unix socket
        #[arg(long)]
        socket: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run { socket } => cmd_run(socket),
        Commands::Ping { socket } => cmd_ping(socket),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_run(socket: Option<PathBuf>) -> Result<(), String> {
    gui::run(socket.as_deref())
}

fn cmd_ping(socket: PathBuf) -> Result<(), String> {
    let mut client = ipc::IpcClient::connect(&socket)?;
    client.ping()?;
    println!("pong");
    Ok(())
}
