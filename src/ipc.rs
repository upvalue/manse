use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// Request sent from client to server
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    /// Check if server is alive
    Ping,
    /// Rename a terminal by UUID
    TermRename { terminal: String, title: String },
    /// Set terminal description by UUID
    TermDesc { terminal: String, description: String },
    /// Move a terminal to a workspace (creates workspace if needed)
    TermToWorkspace { terminal: String, workspace_name: String },
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

/// Server that listens for IPC commands
pub struct IpcServer {
    listener: UnixListener,
    socket_path: PathBuf,
    clients: Vec<UnixStream>,
}

impl IpcServer {
    /// Create a new IPC server at the given socket path.
    /// Returns an error if another instance is already running on this socket.
    pub fn new(socket_path: impl AsRef<Path>) -> Result<Self, String> {
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

        // Create the listener
        let listener = UnixListener::bind(&socket_path)
            .map_err(|e| format!("Failed to bind socket {}: {}", socket_path.display(), e))?;

        // Set non-blocking so we can poll in the event loop
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        Ok(Self {
            listener,
            socket_path,
            clients: Vec::new(),
        })
    }

    /// Poll for incoming connections and commands. Call this each frame.
    /// Returns a list of requests that need to be handled.
    pub fn poll(&mut self) -> Vec<(usize, Request)> {
        let mut requests = Vec::new();

        // Accept new connections
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(true).ok();
                    self.clients.push(stream);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(_) => break,
            }
        }

        // Read from existing clients
        let mut to_remove = Vec::new();
        for (idx, client) in self.clients.iter_mut().enumerate() {
            let mut reader = BufReader::new(client.try_clone().unwrap());
            let mut line = String::new();

            match reader.read_line(&mut line) {
                Ok(0) => {
                    // Connection closed
                    to_remove.push(idx);
                }
                Ok(_) => {
                    if let Ok(request) = serde_json::from_str::<Request>(&line) {
                        requests.push((idx, request));
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available, that's fine
                }
                Err(_) => {
                    to_remove.push(idx);
                }
            }
        }

        // Remove disconnected clients (in reverse order to preserve indices)
        for idx in to_remove.into_iter().rev() {
            self.clients.remove(idx);
        }

        requests
    }

    /// Send a response to a specific client
    pub fn respond(&mut self, client_idx: usize, response: &Response) {
        if let Some(client) = self.clients.get_mut(client_idx) {
            if let Ok(json) = serde_json::to_string(response) {
                let _ = writeln!(client, "{}", json);
                let _ = client.flush();
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Clean up socket file on exit
        let _ = std::fs::remove_file(&self.socket_path);
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
