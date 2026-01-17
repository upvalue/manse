use crate::ipc::{IpcServer, Request, Response};
use crate::terminal::{render_terminal, AlacrittyTerminal, Rgb};
use crate::terminal::renderer::StyledCharacterCache;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};
use std::path::Path;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const FONT_SIZE: u16 = 16;

const BG_COLOR: Color = Color::RGB(0, 0, 0);
const DEFAULT_BG: Rgb = Rgb { r: 0, g: 0, b: 0 };
const INDICATOR_COLOR: Color = Color::RGB(100, 100, 100);
const INDICATOR_ACTIVE_COLOR: Color = Color::RGB(200, 200, 200);
const SCROLL_EASING: f32 = 0.15;
const WIDTH_RATIOS: [f32; 4] = [0.333, 0.5, 0.667, 1.0];

/// Terminal wrapper combining AlacrittyTerminal with window manager state
struct Terminal {
    backend: AlacrittyTerminal,
    width_ratio: f32,
}

impl Terminal {
    fn new(cols: usize, rows: usize, width_ratio: f32) -> Result<Self, String> {
        let backend = AlacrittyTerminal::new(cols as u16, rows as u16)?;
        Ok(Self { backend, width_ratio })
    }

    fn pixel_width(&self, viewport_width: u32) -> u32 {
        (viewport_width as f32 * self.width_ratio) as u32
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.backend.resize(cols as u16, rows as u16);
    }

    fn send_input(&mut self, data: &[u8]) {
        self.backend.write(data);
    }

    fn is_alive(&self) -> bool {
        self.backend.is_alive()
    }
}

struct WindowManager {
    terminals: Vec<Terminal>,
    scroll_offset: f32,
    target_offset: f32,
    focused_index: usize,
    viewport_width: u32,
    viewport_height: u32,
    char_width: u32,
    char_height: u32,
    terminal_count: usize,
}

impl WindowManager {
    fn new(viewport_width: u32, viewport_height: u32, char_width: u32, char_height: u32) -> Result<Self, String> {
        let cols = viewport_width as usize / char_width.max(1) as usize;
        let rows = viewport_height as usize / char_height.max(1) as usize;

        let mut manager = Self {
            terminals: Vec::new(),
            scroll_offset: 0.0,
            target_offset: 0.0,
            focused_index: 0,
            viewport_width,
            viewport_height,
            char_width,
            char_height,
            terminal_count: 0,
        };
        manager.add_terminal(cols.max(1), rows.max(1), 1.0)?;
        Ok(manager)
    }

    fn add_terminal(&mut self, cols: usize, rows: usize, width_ratio: f32) -> Result<(), String> {
        let terminal = Terminal::new(cols, rows, width_ratio)?;
        self.terminals.push(terminal);
        self.terminal_count += 1;
        Ok(())
    }

    fn create_new_terminal(&mut self) -> Result<(), String> {
        let cols = self.viewport_width as usize / self.char_width.max(1) as usize;
        let rows = self.viewport_height as usize / self.char_height.max(1) as usize;
        self.add_terminal(cols.max(1), rows.max(1), 1.0)?;
        self.focused_index = self.terminals.len() - 1;
        self.scroll_to_focused();
        Ok(())
    }

    fn focused_terminal_mut(&mut self) -> Option<&mut Terminal> {
        self.terminals.get_mut(self.focused_index)
    }

    fn terminal_x_position(&self, index: usize) -> f32 {
        let mut x = 0.0;
        for i in 0..index {
            x += self.terminals[i].pixel_width(self.viewport_width) as f32;
        }
        x
    }

    fn scroll_to_focused(&mut self) {
        if self.terminals.is_empty() {
            return;
        }

        let term_x = self.terminal_x_position(self.focused_index);
        let term_width = self.terminals[self.focused_index].pixel_width(self.viewport_width) as f32;
        let term_right = term_x + term_width;
        let view_right = self.target_offset + self.viewport_width as f32;

        if term_x < self.target_offset {
            self.target_offset = term_x;
        } else if term_right > view_right {
            self.target_offset = term_right - self.viewport_width as f32;
        }

        let max_scroll = self.total_content_width() - self.viewport_width as f32;
        self.target_offset = self.target_offset.max(0.0).min(max_scroll.max(0.0));
    }

    fn total_content_width(&self) -> f32 {
        self.terminals
            .iter()
            .map(|t| t.pixel_width(self.viewport_width) as f32)
            .sum()
    }

    fn focus_next(&mut self) {
        if self.focused_index < self.terminals.len() - 1 {
            self.focused_index += 1;
            self.scroll_to_focused();
        }
    }

    fn focus_prev(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
            self.scroll_to_focused();
        }
    }

    fn grow_focused(&mut self) {
        if let Some(terminal) = self.terminals.get_mut(self.focused_index) {
            let current = terminal.width_ratio;
            for &ratio in WIDTH_RATIOS.iter() {
                if ratio > current + 0.01 {
                    terminal.width_ratio = ratio;
                    self.resize_terminal_grid(self.focused_index);
                    self.scroll_to_focused();
                    break;
                }
            }
        }
    }

    fn shrink_focused(&mut self) {
        if let Some(terminal) = self.terminals.get_mut(self.focused_index) {
            let current = terminal.width_ratio;
            for &ratio in WIDTH_RATIOS.iter().rev() {
                if ratio < current - 0.01 {
                    terminal.width_ratio = ratio;
                    self.resize_terminal_grid(self.focused_index);
                    self.scroll_to_focused();
                    break;
                }
            }
        }
    }

    fn resize_terminal_grid(&mut self, index: usize) {
        if let Some(terminal) = self.terminals.get_mut(index) {
            let pixel_width = terminal.pixel_width(self.viewport_width);
            let cols = pixel_width as usize / self.char_width.max(1) as usize;
            let rows = self.viewport_height as usize / self.char_height.max(1) as usize;
            terminal.resize(cols.max(1), rows.max(1));
        }
    }

    fn update(&mut self) {
        // Animate scrolling
        let diff = self.target_offset - self.scroll_offset;
        if diff.abs() > 0.5 {
            self.scroll_offset += diff * SCROLL_EASING;
        } else {
            self.scroll_offset = self.target_offset;
        }
        // Terminal content is updated by its background event loop
        // We sync during rendering to get the latest content
    }

    fn resize_viewport(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;

        for i in 0..self.terminals.len() {
            self.resize_terminal_grid(i);
        }

        self.scroll_to_focused();
        self.scroll_offset = self.target_offset;
    }
}

fn find_font_path() -> Result<String, String> {
    let font_paths = [
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/SFMono-Regular.otf",
        "/Library/Fonts/SF-Mono-Regular.otf",
        "/System/Library/Fonts/Courier.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "assets/fonts/font.ttf",
    ];

    for path in font_paths {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    Err("No suitable monospace font found".to_string())
}

fn render_window_manager<'a>(
    canvas: &mut Canvas<Window>,
    manager: &mut WindowManager,
    char_cache: &mut StyledCharacterCache<'a>,
    font: &Font,
    texture_creator: &'a TextureCreator<WindowContext>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<(), String> {
    let scroll_offset = manager.scroll_offset as i32;

    let mut x_pos: f32 = 0.0;
    for i in 0..manager.terminals.len() {
        let term_width = manager.terminals[i].pixel_width(viewport_width);
        let x_offset = x_pos as i32 - scroll_offset;

        // Skip terminals completely off-screen
        if x_offset > viewport_width as i32 || x_offset + (term_width as i32) < 0 {
            x_pos += term_width as f32;
            continue;
        }

        // Draw focus indicator
        if i == manager.focused_index {
            canvas.set_draw_color(INDICATOR_ACTIVE_COLOR);
            canvas.fill_rect(Rect::new(x_offset, 0, 2, viewport_height))?;
        }

        // Sync terminal content and render
        let content = manager.terminals[i].backend.sync();
        render_terminal(
            canvas,
            content,
            char_cache,
            font,
            texture_creator,
            (x_offset, 0),
            DEFAULT_BG,
            viewport_width,
        )?;

        x_pos += term_width as f32;
    }

    render_position_indicators(canvas, manager, viewport_width, viewport_height)?;

    Ok(())
}

fn render_position_indicators(
    canvas: &mut Canvas<Window>,
    manager: &WindowManager,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<(), String> {
    let num_terminals = manager.terminals.len();
    if num_terminals <= 1 {
        return Ok(());
    }

    let dot_radius: i32 = 4;
    let dot_spacing: i32 = 16;
    let total_width = (num_terminals as i32 - 1) * dot_spacing;
    let start_x = (viewport_width as i32 - total_width) / 2;
    let y = viewport_height as i32 - 20;

    for i in 0..num_terminals {
        let x = start_x + (i as i32 * dot_spacing);
        let is_active = i == manager.focused_index;

        if is_active {
            canvas.set_draw_color(INDICATOR_ACTIVE_COLOR);
        } else {
            canvas.set_draw_color(INDICATOR_COLOR);
        }

        canvas.fill_rect(Rect::new(
            x - dot_radius,
            y - dot_radius,
            (dot_radius * 2) as u32,
            (dot_radius * 2) as u32,
        ))?;
    }

    Ok(())
}

/// Handle an IPC request and return a response
fn handle_ipc_request(_manager: &WindowManager, request: Request) -> Response {
    match request {
        Request::Ping => Response::ok(),
    }
}

/// Encode special keys to terminal escape sequences
fn encode_special_key(keycode: Keycode) -> Option<&'static str> {
    match keycode {
        Keycode::Up => Some("\x1b[A"),
        Keycode::Down => Some("\x1b[B"),
        Keycode::Right => Some("\x1b[C"),
        Keycode::Left => Some("\x1b[D"),
        Keycode::Home => Some("\x1b[H"),
        Keycode::End => Some("\x1b[F"),
        Keycode::PageUp => Some("\x1b[5~"),
        Keycode::PageDown => Some("\x1b[6~"),
        Keycode::Insert => Some("\x1b[2~"),
        Keycode::Delete => Some("\x1b[3~"),
        Keycode::Backspace => Some("\x7f"),
        Keycode::Return => Some("\r"),
        Keycode::Tab => Some("\t"),
        Keycode::F1 => Some("\x1bOP"),
        Keycode::F2 => Some("\x1bOQ"),
        Keycode::F3 => Some("\x1bOR"),
        Keycode::F4 => Some("\x1bOS"),
        Keycode::F5 => Some("\x1b[15~"),
        Keycode::F6 => Some("\x1b[17~"),
        Keycode::F7 => Some("\x1b[18~"),
        Keycode::F8 => Some("\x1b[19~"),
        Keycode::F9 => Some("\x1b[20~"),
        Keycode::F10 => Some("\x1b[21~"),
        Keycode::F11 => Some("\x1b[23~"),
        Keycode::F12 => Some("\x1b[24~"),
        _ => None,
    }
}

/// Encode Ctrl+key combinations to control characters
fn encode_ctrl_key(keycode: Keycode) -> Option<&'static str> {
    match keycode {
        Keycode::A => Some("\x01"),
        Keycode::B => Some("\x02"),
        Keycode::C => Some("\x03"),
        Keycode::D => Some("\x04"),
        Keycode::E => Some("\x05"),
        Keycode::F => Some("\x06"),
        Keycode::G => Some("\x07"),
        // H, L, N are handled as WM shortcuts
        Keycode::I => Some("\x09"), // Tab
        Keycode::J => Some("\x0a"), // Newline
        Keycode::K => Some("\x0b"),
        Keycode::M => Some("\x0d"), // Enter
        Keycode::O => Some("\x0f"),
        Keycode::P => Some("\x10"),
        Keycode::Q => Some("\x11"),
        Keycode::R => Some("\x12"),
        Keycode::S => Some("\x13"),
        Keycode::T => Some("\x14"),
        Keycode::U => Some("\x15"),
        Keycode::V => Some("\x16"),
        Keycode::W => Some("\x17"),
        Keycode::X => Some("\x18"),
        Keycode::Y => Some("\x19"),
        Keycode::Z => Some("\x1a"),
        Keycode::LeftBracket => Some("\x1b"), // Escape
        Keycode::Backslash => Some("\x1c"),
        Keycode::RightBracket => Some("\x1d"),
        _ => None,
    }
}

/// Run the GUI application with an optional IPC socket
pub fn run(socket_path: Option<&Path>) -> Result<(), String> {
    // Set up IPC server if socket path provided
    let mut ipc_server = match socket_path {
        Some(path) => Some(IpcServer::new(path)?),
        None => None,
    };

    if let Some(ref server) = ipc_server {
        println!("IPC socket: {}", server.socket_path().display());
    }

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let window = video_subsystem
        .window("Manse Terminal", WINDOW_WIDTH, WINDOW_HEIGHT)
        .maximized()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    let info = canvas.info();
    println!("Renderer: {}", info.name);
    println!(
        "Accelerated: {}",
        info.name.contains("metal") || info.name.contains("opengl")
    );
    println!("Controls: Ctrl+N (new), Ctrl+H/L (navigate), Ctrl+,/. (resize)");

    let texture_creator = canvas.texture_creator();

    let font_path = find_font_path()?;
    println!("Using font: {}", font_path);
    let font = ttf_context.load_font(&font_path, FONT_SIZE)?;

    let mut char_cache = StyledCharacterCache::new();
    char_cache.init_dimensions(&font);

    let (win_width, win_height) = canvas.output_size()?;
    let mut window_manager = WindowManager::new(
        win_width,
        win_height,
        char_cache.char_width,
        char_cache.char_height,
    )?;

    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        // Poll IPC socket for commands
        if let Some(ref mut server) = ipc_server {
            let requests = server.poll();
            for (client_idx, request) in requests {
                let response = handle_ipc_request(&window_manager, request);
                server.respond(client_idx, &response);
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                // Window manager shortcuts (Ctrl+key)
                Event::KeyDown {
                    keycode: Some(Keycode::N),
                    keymod,
                    ..
                } if keymod.contains(Mod::LCTRLMOD)
                    || keymod.contains(Mod::RCTRLMOD)
                    || keymod.contains(Mod::LGUIMOD)
                    || keymod.contains(Mod::RGUIMOD) =>
                {
                    if let Err(e) = window_manager.create_new_terminal() {
                        eprintln!("Failed to create terminal: {}", e);
                    }
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Period),
                    keymod,
                    ..
                } if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) => {
                    window_manager.grow_focused();
                }

                Event::KeyDown {
                    keycode: Some(Keycode::Comma),
                    keymod,
                    ..
                } if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) => {
                    window_manager.shrink_focused();
                }

                Event::KeyDown {
                    keycode: Some(Keycode::L),
                    keymod,
                    ..
                } if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) => {
                    window_manager.focus_next();
                }

                Event::KeyDown {
                    keycode: Some(Keycode::H),
                    keymod,
                    ..
                } if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) => {
                    window_manager.focus_prev();
                }

                // Text input - forward to terminal
                Event::TextInput { text, .. } => {
                    if let Some(term) = window_manager.focused_terminal_mut() {
                        term.send_input(text.as_bytes());
                    }
                }

                // Special keys - forward encoded sequences to terminal
                Event::KeyDown {
                    keycode: Some(keycode),
                    keymod,
                    ..
                } => {
                    // Skip if it's a WM shortcut (handled above)
                    let is_ctrl = keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD);
                    if is_ctrl {
                        // Forward Ctrl+key as control sequence
                        if let Some(term) = window_manager.focused_terminal_mut() {
                            if let Some(seq) = encode_ctrl_key(keycode) {
                                term.send_input(seq.as_bytes());
                            }
                        }
                    } else if let Some(seq) = encode_special_key(keycode) {
                        // Forward special keys (arrows, etc.)
                        if let Some(term) = window_manager.focused_terminal_mut() {
                            term.send_input(seq.as_bytes());
                        }
                    }
                }

                Event::Window {
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } => {
                    window_manager.resize_viewport(w as u32, h as u32);
                }

                _ => {}
            }
        }

        window_manager.update();

        canvas.set_draw_color(BG_COLOR);
        canvas.clear();

        let (win_width, win_height) = canvas.output_size()?;
        render_window_manager(
            &mut canvas,
            &mut window_manager,
            &mut char_cache,
            &font,
            &texture_creator,
            win_width,
            win_height,
        )?;

        canvas.present();
    }

    Ok(())
}
