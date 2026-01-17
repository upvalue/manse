use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator, TextureQuery};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;

use crate::terminal::content::{CellFlags, RenderableContent, Rgb};

/// Cache key for styled character textures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    c: char,
    fg: Rgb,
    bold: bool,
}

/// Character cache with color support
pub struct StyledCharacterCache<'a> {
    textures: HashMap<CacheKey, sdl2::render::Texture<'a>>,
    pub char_width: u32,
    pub char_height: u32,
}

impl<'a> StyledCharacterCache<'a> {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            char_width: 0,
            char_height: 0,
        }
    }

    pub fn init_dimensions(&mut self, font: &Font) {
        if let Ok(metrics) = font.size_of_char('M') {
            self.char_width = metrics.0;
            self.char_height = metrics.1;
        }
    }

    pub fn get_or_create(
        &mut self,
        c: char,
        fg: Rgb,
        flags: CellFlags,
        font: &Font,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> Result<&sdl2::render::Texture<'a>, String> {
        let key = CacheKey {
            c,
            fg,
            bold: flags.contains(CellFlags::BOLD),
        };

        if !self.textures.contains_key(&key) {
            let color = Color::RGB(fg.r, fg.g, fg.b);
            let surface = font
                .render(&c.to_string())
                .blended(color)
                .map_err(|e| e.to_string())?;
            let texture = texture_creator
                .create_texture_from_surface(&surface)
                .map_err(|e| e.to_string())?;

            if self.char_width == 0 {
                let TextureQuery { width, height, .. } = texture.query();
                self.char_width = width;
                self.char_height = height;
            }

            self.textures.insert(key, texture);
        }

        Ok(self.textures.get(&key).unwrap())
    }
}

/// Render terminal content to SDL2 canvas
pub fn render_terminal<'a>(
    canvas: &mut Canvas<Window>,
    content: &RenderableContent,
    cache: &mut StyledCharacterCache<'a>,
    font: &Font,
    texture_creator: &'a TextureCreator<WindowContext>,
    origin: (i32, i32),
    default_bg: Rgb,
    viewport_width: u32,
) -> Result<(), String> {
    let (ox, oy) = origin;
    let cw = cache.char_width as i32;
    let ch = cache.char_height as i32;

    for (idx, cell) in content.cells.iter().enumerate() {
        let col = (idx % content.cols) as i32;
        let row = (idx / content.cols) as i32;
        let x = ox + col * cw;
        let y = oy + row * ch;

        // Skip if outside viewport (horizontal clipping)
        if x + cw < 0 || x > viewport_width as i32 {
            continue;
        }

        // Skip hidden cells
        if cell.flags.contains(CellFlags::HIDDEN) {
            continue;
        }

        // Draw background if different from default
        if cell.bg != default_bg {
            canvas.set_draw_color(cell.bg.to_sdl());
            canvas.fill_rect(Rect::new(x, y, cw as u32, ch as u32))?;
        }

        // Draw character
        if cell.c != ' ' && cell.c != '\0' && !cell.c.is_control() {
            let texture = cache.get_or_create(cell.c, cell.fg, cell.flags, font, texture_creator)?;
            let TextureQuery { width, height, .. } = texture.query();
            canvas.copy(texture, None, Rect::new(x, y, width, height))?;
        }

        // Draw underline
        if cell.flags.contains(CellFlags::UNDERLINE) {
            canvas.set_draw_color(cell.fg.to_sdl());
            let underline_y = y + ch - 2;
            canvas.draw_line((x, underline_y), (x + cw, underline_y))?;
        }

        // Draw strikethrough
        if cell.flags.contains(CellFlags::STRIKETHROUGH) {
            canvas.set_draw_color(cell.fg.to_sdl());
            let strike_y = y + ch / 2;
            canvas.draw_line((x, strike_y), (x + cw, strike_y))?;
        }
    }

    // Draw cursor
    if let Some(cursor) = &content.cursor {
        if cursor.visible {
            let cx = ox + cursor.col as i32 * cw;
            let cy = oy + cursor.row as i32 * ch;

            // Only draw if cursor is visible in viewport
            if cx >= 0 && cx < viewport_width as i32 {
                canvas.set_draw_color(Color::RGBA(200, 200, 200, 200));

                match cursor.shape {
                    crate::terminal::content::CursorShape::Block => {
                        // Semi-transparent block cursor
                        canvas.fill_rect(Rect::new(cx, cy, cw as u32, ch as u32))?;
                    }
                    crate::terminal::content::CursorShape::Beam => {
                        // Vertical line cursor
                        canvas.fill_rect(Rect::new(cx, cy, 2, ch as u32))?;
                    }
                    crate::terminal::content::CursorShape::Underline => {
                        // Underline cursor
                        canvas.fill_rect(Rect::new(cx, cy + ch - 2, cw as u32, 2))?;
                    }
                }
            }
        }
    }

    Ok(())
}
