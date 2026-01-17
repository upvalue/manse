mod app;
mod ipc;

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
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },
    /// Ping a running instance
    Ping {
        /// Path to IPC socket
        #[arg(short, long)]
        socket: PathBuf,
    },
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { socket } => {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([1200.0, 800.0])
                    .with_min_inner_size([400.0, 300.0])
                    .with_maximized(true),
                ..Default::default()
            };

            eframe::run_native(
                "Manse Terminal",
                options,
                Box::new(move |cc| Ok(Box::new(app::App::new(cc, socket)))),
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
    }
}
