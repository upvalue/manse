use std::borrow::Cow;
use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

use alacritty_terminal::event::{Event, EventListener, Notify, OnResize, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, Msg, State as EventLoopState};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::tty::{self, EventedReadWrite, Options as PtyOptions, Pty};

use crate::terminal::content::{ColorPalette, RenderableContent};

/// Event proxy that sends terminal events back to main thread
#[derive(Clone)]
pub struct EventProxy {
    sender: Sender<TerminalEvent>,
}

impl EventProxy {
    fn new(sender: Sender<TerminalEvent>) -> Self {
        Self { sender }
    }
}

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.sender.send(TerminalEvent::AlacrittyEvent(event));
    }
}

/// Events sent from the PTY event loop
#[derive(Debug)]
pub enum TerminalEvent {
    AlacrittyEvent(Event),
    Exit,
}

/// PTY writer handle for sending input
pub struct PtyWriter {
    writer: Box<dyn Write + Send>,
}

impl PtyWriter {
    pub fn write(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }
}

/// Wrapper around alacritty_terminal for real terminal emulation
pub struct AlacrittyTerminal {
    term: Arc<FairMutex<Term<EventProxy>>>,
    pty_writer: PtyWriter,
    event_rx: Receiver<TerminalEvent>,
    #[allow(dead_code)]
    _event_loop_handle: JoinHandle<(EventLoop<Pty, EventProxy>, EventLoopState)>,
    notifier: Notifier,
    cols: u16,
    rows: u16,
    palette: ColorPalette,
    content: RenderableContent,
    exited: bool,
}

/// Notifier to wake up the event loop
#[derive(Clone)]
pub struct Notifier(Sender<Msg>);

impl Notify for Notifier {
    fn notify<B: Into<Cow<'static, [u8]>>>(&self, bytes: B) {
        let _ = self.0.send(Msg::Input(bytes.into()));
    }
}

impl OnResize for Notifier {
    fn on_resize(&mut self, size: WindowSize) {
        let _ = self.0.send(Msg::Resize(size));
    }
}

impl AlacrittyTerminal {
    /// Create a new terminal with PTY running the default shell
    pub fn new(cols: u16, rows: u16) -> Result<Self, String> {
        let (event_tx, event_rx) = mpsc::channel();
        let event_proxy = EventProxy::new(event_tx);

        // Create terminal size
        let term_size = TermSize::new(cols as usize, rows as usize);

        // Create the terminal
        let term_config = TermConfig::default();
        let term = Term::new(term_config, &term_size, event_proxy.clone());
        let term = Arc::new(FairMutex::new(term));

        // Set up PTY options with proper TERM environment
        // Inherit current environment and override TERM
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut env: std::collections::HashMap<String, String> = std::env::vars().collect();
        env.insert("TERM".to_string(), "xterm-256color".to_string());

        let pty_options = PtyOptions {
            shell: Some(tty::Shell::new(shell, vec![])),
            working_directory: std::env::current_dir().ok(),
            env,
            ..Default::default()
        };

        // Create window size for PTY
        let window_size = WindowSize {
            num_cols: cols,
            num_lines: rows,
            cell_width: 1,
            cell_height: 1,
        };

        // Create the PTY
        let mut pty = tty::new(&pty_options, window_size, 0)
            .map_err(|e| format!("Failed to create PTY: {}", e))?;

        // Get writer before moving pty - we need to clone the writer FD
        // The PTY writer is typically a file descriptor we can clone
        let pty_fd = pty.writer().try_clone()
            .map_err(|e| format!("Failed to clone PTY writer: {}", e))?;
        let pty_writer = PtyWriter {
            writer: Box::new(pty_fd),
        };

        // Create event loop channel
        let (msg_tx, _msg_rx) = mpsc::channel();
        let notifier = Notifier(msg_tx);

        // Create and spawn event loop
        let event_loop = EventLoop::new(
            term.clone(),
            event_proxy,
            pty,
            false, // bracketed paste
            false, // kitty keyboard
        ).map_err(|e| format!("Failed to create event loop: {}", e))?;

        let loop_handle = event_loop.spawn();

        let palette = ColorPalette::default();
        let content = {
            let term_lock = term.lock();
            RenderableContent::from_term(&term_lock, &palette)
        };

        Ok(Self {
            term,
            pty_writer,
            event_rx,
            _event_loop_handle: loop_handle,
            notifier,
            cols,
            rows,
            palette,
            content,
            exited: false,
        })
    }

    /// Sync terminal state and return renderable content
    pub fn sync(&mut self) -> &RenderableContent {
        // Process any pending events
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                TerminalEvent::AlacrittyEvent(Event::Exit) => {
                    self.exited = true;
                }
                TerminalEvent::AlacrittyEvent(Event::PtyWrite(data)) => {
                    // Write terminal responses back to PTY (e.g., device attribute queries)
                    self.pty_writer.write(data.as_bytes());
                }
                TerminalEvent::Exit => {
                    self.exited = true;
                }
                _ => {}
            }
        }

        // Extract current content from terminal
        let term = self.term.lock();
        self.content = RenderableContent::from_term(&term, &self.palette);
        &self.content
    }

    /// Send input bytes to the PTY
    pub fn write(&mut self, input: &[u8]) {
        self.pty_writer.write(input);
    }

    /// Resize the terminal and PTY
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }

        self.cols = cols;
        self.rows = rows;

        let size = WindowSize {
            num_cols: cols,
            num_lines: rows,
            cell_width: 1,
            cell_height: 1,
        };

        // Resize the terminal grid
        {
            let mut term = self.term.lock();
            let term_size = TermSize::new(cols as usize, rows as usize);
            term.resize(term_size);
        }

        // Notify the PTY of the resize
        let mut notifier = self.notifier.clone();
        notifier.on_resize(size);
    }

    /// Check if the shell process has exited
    pub fn is_alive(&self) -> bool {
        !self.exited
    }

    /// Get current dimensions
    pub fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Get a reference to the current content without syncing
    pub fn content(&self) -> &RenderableContent {
        &self.content
    }
}
