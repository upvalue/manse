use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor};
use bitflags::bitflags;

/// RGB color for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_sdl(&self) -> sdl2::pixels::Color {
        sdl2::pixels::Color::RGB(self.r, self.g, self.b)
    }
}

impl Default for Rgb {
    fn default() -> Self {
        Self::new(220, 220, 220)
    }
}

/// Default terminal color palette (ANSI 256 colors)
pub struct ColorPalette {
    colors: [Rgb; 256],
    foreground: Rgb,
    background: Rgb,
}

impl Default for ColorPalette {
    fn default() -> Self {
        let mut colors = [Rgb::new(0, 0, 0); 256];

        // Standard colors (0-7)
        colors[0] = Rgb::new(0, 0, 0);       // Black
        colors[1] = Rgb::new(205, 49, 49);   // Red
        colors[2] = Rgb::new(13, 188, 121);  // Green
        colors[3] = Rgb::new(229, 229, 16);  // Yellow
        colors[4] = Rgb::new(36, 114, 200);  // Blue
        colors[5] = Rgb::new(188, 63, 188);  // Magenta
        colors[6] = Rgb::new(17, 168, 205);  // Cyan
        colors[7] = Rgb::new(229, 229, 229); // White

        // Bright colors (8-15)
        colors[8] = Rgb::new(102, 102, 102);  // Bright Black
        colors[9] = Rgb::new(241, 76, 76);    // Bright Red
        colors[10] = Rgb::new(35, 209, 139);  // Bright Green
        colors[11] = Rgb::new(245, 245, 67);  // Bright Yellow
        colors[12] = Rgb::new(59, 142, 234);  // Bright Blue
        colors[13] = Rgb::new(214, 112, 214); // Bright Magenta
        colors[14] = Rgb::new(41, 184, 219);  // Bright Cyan
        colors[15] = Rgb::new(255, 255, 255); // Bright White

        // 216 color cube (16-231)
        for r in 0..6 {
            for g in 0..6 {
                for b in 0..6 {
                    let idx = 16 + r * 36 + g * 6 + b;
                    let r_val = if r == 0 { 0 } else { r * 40 + 55 };
                    let g_val = if g == 0 { 0 } else { g * 40 + 55 };
                    let b_val = if b == 0 { 0 } else { b * 40 + 55 };
                    colors[idx] = Rgb::new(r_val as u8, g_val as u8, b_val as u8);
                }
            }
        }

        // Grayscale (232-255)
        for i in 0..24 {
            let val = (i * 10 + 8) as u8;
            colors[232 + i] = Rgb::new(val, val, val);
        }

        Self {
            colors,
            foreground: Rgb::new(220, 220, 220),
            background: Rgb::new(0, 0, 0),
        }
    }
}

impl ColorPalette {
    pub fn resolve(&self, color: AnsiColor) -> Rgb {
        match color {
            AnsiColor::Named(named) => self.resolve_named(named),
            AnsiColor::Spec(rgb) => Rgb::new(rgb.r, rgb.g, rgb.b),
            AnsiColor::Indexed(idx) => self.colors[idx as usize],
        }
    }

    fn resolve_named(&self, named: NamedColor) -> Rgb {
        match named {
            NamedColor::Foreground => self.foreground,
            NamedColor::Background => self.background,
            NamedColor::Cursor => Rgb::new(200, 200, 200),
            NamedColor::Black => self.colors[0],
            NamedColor::Red => self.colors[1],
            NamedColor::Green => self.colors[2],
            NamedColor::Yellow => self.colors[3],
            NamedColor::Blue => self.colors[4],
            NamedColor::Magenta => self.colors[5],
            NamedColor::Cyan => self.colors[6],
            NamedColor::White => self.colors[7],
            NamedColor::BrightBlack => self.colors[8],
            NamedColor::BrightRed => self.colors[9],
            NamedColor::BrightGreen => self.colors[10],
            NamedColor::BrightYellow => self.colors[11],
            NamedColor::BrightBlue => self.colors[12],
            NamedColor::BrightMagenta => self.colors[13],
            NamedColor::BrightCyan => self.colors[14],
            NamedColor::BrightWhite => self.colors[15],
            _ => self.foreground,
        }
    }

    pub fn foreground(&self) -> Rgb {
        self.foreground
    }

    pub fn background(&self) -> Rgb {
        self.background
    }
}

bitflags! {
    /// Cell styling flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CellFlags: u8 {
        const BOLD = 0x01;
        const ITALIC = 0x02;
        const UNDERLINE = 0x04;
        const DIM = 0x08;
        const INVERSE = 0x10;
        const STRIKETHROUGH = 0x20;
        const HIDDEN = 0x40;
    }
}

impl From<Flags> for CellFlags {
    fn from(flags: Flags) -> Self {
        let mut result = CellFlags::empty();
        if flags.contains(Flags::BOLD) {
            result |= CellFlags::BOLD;
        }
        if flags.contains(Flags::ITALIC) {
            result |= CellFlags::ITALIC;
        }
        if flags.contains(Flags::UNDERLINE) || flags.contains(Flags::DOUBLE_UNDERLINE) {
            result |= CellFlags::UNDERLINE;
        }
        if flags.contains(Flags::DIM) {
            result |= CellFlags::DIM;
        }
        if flags.contains(Flags::INVERSE) {
            result |= CellFlags::INVERSE;
        }
        if flags.contains(Flags::STRIKEOUT) {
            result |= CellFlags::STRIKETHROUGH;
        }
        if flags.contains(Flags::HIDDEN) {
            result |= CellFlags::HIDDEN;
        }
        result
    }
}

/// A single renderable cell
#[derive(Debug, Clone)]
pub struct RenderableCell {
    pub c: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: CellFlags,
    pub wide: bool,
}

impl Default for RenderableCell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: Rgb::new(220, 220, 220),
            bg: Rgb::new(0, 0, 0),
            flags: CellFlags::empty(),
            wide: false,
        }
    }
}

/// Cursor shape for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Underline,
    Beam,
}

impl Default for CursorShape {
    fn default() -> Self {
        Self::Block
    }
}

/// Cursor state for rendering
#[derive(Debug, Clone)]
pub struct CursorState {
    pub col: usize,
    pub row: usize,
    pub shape: CursorShape,
    pub visible: bool,
}

/// Content extracted from terminal for rendering
#[derive(Debug, Clone)]
pub struct RenderableContent {
    pub cells: Vec<RenderableCell>,
    pub cols: usize,
    pub rows: usize,
    pub cursor: Option<CursorState>,
}

impl RenderableContent {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cells: vec![RenderableCell::default(); cols * rows],
            cols,
            rows,
            cursor: None,
        }
    }

    /// Extract renderable content from an alacritty terminal
    pub fn from_term<T>(term: &Term<T>, palette: &ColorPalette) -> Self {
        let grid = term.grid();
        let cols = grid.columns();
        let rows = grid.screen_lines();
        let mut cells = Vec::with_capacity(cols * rows);

        // Iterate over displayed lines (from top to bottom)
        for line in 0..rows {
            let row = &grid[Line(line as i32)];
            for col in 0..cols {
                let cell = &row[Column(col)];
                let flags: CellFlags = cell.flags.into();

                let (fg, bg) = if flags.contains(CellFlags::INVERSE) {
                    (
                        palette.resolve(cell.bg),
                        palette.resolve(cell.fg),
                    )
                } else {
                    (
                        palette.resolve(cell.fg),
                        palette.resolve(cell.bg),
                    )
                };

                let wide = cell.flags.contains(Flags::WIDE_CHAR);

                cells.push(RenderableCell {
                    c: cell.c,
                    fg,
                    bg,
                    flags,
                    wide,
                });
            }
        }

        // Extract cursor state
        let cursor_point = term.grid().cursor.point;
        let cursor = if term.mode().contains(TermMode::SHOW_CURSOR) {
            Some(CursorState {
                col: cursor_point.column.0,
                row: cursor_point.line.0 as usize,
                shape: CursorShape::Block,
                visible: true,
            })
        } else {
            None
        };

        Self {
            cells,
            cols,
            rows,
            cursor,
        }
    }

    /// Get a cell at a specific position
    pub fn get(&self, col: usize, row: usize) -> Option<&RenderableCell> {
        if col < self.cols && row < self.rows {
            self.cells.get(row * self.cols + col)
        } else {
            None
        }
    }
}
