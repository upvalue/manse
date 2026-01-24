use eframe::egui;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Request sent from client to server
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    /// Check if server is alive
    Ping,
    /// Trigger a restart (exec with state preservation)
    Restart,
    /// Rename a terminal by ID
    TermRename { terminal: String, title: String },
    /// Set terminal description by ID
    TermDesc { terminal: String, description: String },
    /// Set terminal emoji icon by ID
    TermEmoji { terminal: String, emoji: String },
    /// Move a terminal to a workspace (creates workspace if needed)
    TermToWorkspace { terminal: String, workspace_name: String },
    /// Set notification on a terminal (cleared when focused)
    TermNotify { terminal: String },
}

/// Response sent from server to client
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
}

impl Response {
    pub fn ok() -> Self {
        Self {
            ok: true,
            error: None,
            result: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
            result: None,
        }
    }
}

/// A pending IPC request with a channel to send the response back
pub struct PendingRequest {
    pub request: Request,
    response_tx: Sender<Response>,
}

impl PendingRequest {
    /// Send a response back to the client
    pub fn respond(self, response: Response) {
        let _ = self.response_tx.send(response);
    }
}

/// Handle for the main thread to receive IPC requests
pub struct IpcHandle {
    request_rx: Receiver<PendingRequest>,
    _socket_path: PathBuf,
}

impl IpcHandle {
    /// Poll for pending requests (non-blocking)
    pub fn poll(&self) -> Vec<PendingRequest> {
        let mut requests = Vec::new();
        while let Ok(req) = self.request_rx.try_recv() {
            requests.push(req);
        }
        requests
    }
}

/// Start the IPC server in a background thread.
/// Returns a handle for the main thread to receive requests.
pub fn start_ipc_server(
    socket_path: impl AsRef<Path>,
    ctx: egui::Context,
) -> Result<IpcHandle, String> {
    let socket_path = socket_path.as_ref().to_path_buf();

    // Check if socket already exists
    if socket_path.exists() {
        // Try to connect - if successful, another instance is running
        match UnixStream::connect(&socket_path) {
            Ok(_) => {
                return Err(format!(
                    "Another instance is already running on socket: {}",
                    socket_path.display()
                ));
            }
            Err(_) => {
                // Stale socket file, remove it
                std::fs::remove_file(&socket_path).map_err(|e| {
                    format!(
                        "Failed to remove stale socket {}: {}",
                        socket_path.display(),
                        e
                    )
                })?;
            }
        }
    }

    // Create the listener (blocking mode for the background thread)
    let listener = UnixListener::bind(&socket_path)
        .map_err(|e| format!("Failed to bind socket {}: {}", socket_path.display(), e))?;

    log::info!("IPC server listening on: {}", socket_path.display());

    let (request_tx, request_rx) = mpsc::channel();
    let socket_path_clone = socket_path.clone();

    thread::spawn(move || {
        // Handle cleanup on thread exit
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _cleanup = Cleanup(socket_path_clone);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let request_tx = request_tx.clone();
                    let ctx = ctx.clone();

                    // Handle each client in its own thread for concurrent connections
                    thread::spawn(move || {
                        handle_client(stream, request_tx, ctx);
                    });
                }
                Err(e) => {
                    log::error!("IPC accept error: {}", e);
                }
            }
        }
    });

    Ok(IpcHandle {
        request_rx,
        _socket_path: socket_path,
    })
}

fn handle_client(stream: UnixStream, request_tx: Sender<PendingRequest>, ctx: egui::Context) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // Connection closed
            Ok(_) => {
                if let Ok(request) = serde_json::from_str::<Request>(&line) {
                    // Create a oneshot-style channel for the response
                    let (response_tx, response_rx) = mpsc::channel();

                    let pending = PendingRequest {
                        request,
                        response_tx,
                    };

                    // Send to main thread and request repaint
                    if request_tx.send(pending).is_err() {
                        break; // Main thread gone
                    }
                    ctx.request_repaint();

                    // Wait for response from main thread
                    match response_rx.recv() {
                        Ok(response) => {
                            if let Ok(json) = serde_json::to_string(&response) {
                                if writeln!(writer, "{}", json).is_err() {
                                    break;
                                }
                                if writer.flush().is_err() {
                                    break;
                                }
                            }
                        }
                        Err(_) => break, // Main thread dropped the sender
                    }
                }
            }
            Err(_) => break,
        }
    }
}

/// Client for sending commands to a running instance
pub struct IpcClient {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl IpcClient {
    /// Connect to a running instance
    pub fn connect(socket_path: impl AsRef<Path>) -> Result<Self, String> {
        let socket_path = socket_path.as_ref();
        let stream = UnixStream::connect(socket_path).map_err(|e| {
            format!(
                "Failed to connect to socket {}: {}",
                socket_path.display(),
                e
            )
        })?;
        let reader = BufReader::new(stream.try_clone().unwrap());
        Ok(Self { stream, reader })
    }

    /// Send a request and wait for response
    pub fn request(&mut self, req: &Request) -> Result<Response, String> {
        let json = serde_json::to_string(req).map_err(|e| format!("Failed to serialize: {}", e))?;
        writeln!(self.stream, "{}", json).map_err(|e| format!("Failed to send: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read response: {}", e))?;

        serde_json::from_str(&line).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Send a ping and check if server is alive
    pub fn ping(&mut self) -> Result<(), String> {
        let response = self.request(&Request::Ping)?;
        if response.ok {
            Ok(())
        } else {
            Err(response.error.unwrap_or_else(|| "Unknown error".into()))
        }
    }
}
