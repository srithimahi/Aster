use crate::terminal::Terminal;
use fontdue::{Font, FontSettings};

pub struct Renderer {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
    font: Font,
}

impl Renderer {
    pub fn  new(width: u32, height: u32) -> Self {
        let pixels = vec![0; (width * height) as usize];

        let font_bytes = include_bytes!(
            "../assets/fonts/PTMono-Regular.ttf"
        );

        let font = Font::from_bytes(
            font_bytes as &[u8],
            FontSettings::default(),
        )
        .expect("Failed to load Aster font");

        Self {
            width,
            height,
            pixels,
            font,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixels.resize((width * height) as usize, 0);
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    pub fn draw_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: u32,
    ) {
        let end_x = (x + width).min(self.width);
        let end_y = (y + height).min(self.height);

        for pixel_y in y..end_y {
            for pixel_x in x..end_x {
                let index = 
                    (pixel_y * self.width + pixel_x) as usize;

                self.pixels[index] = color;
            }
        }
    }

    pub fn draw_char(
        &mut self,
        character: char,
        x: u32,
        y: u32,
        size: f32,
        color: u32,
    ) {
        let (metrics, bitmap) =
            self.font.rasterize(character, size);

        for glyph_y in 0..metrics.height {
            for glyph_x in 0..metrics.width {
                let source_index = 
                    glyph_y * metrics.width + glyph_x;

                let coverage = bitmap[source_index];

                if coverage == 0 {
                    continue;
                }

                let target_x = x + glyph_x as u32;
                let target_y = y + glyph_y as u32;

                if target_x >= self.width || target_y >= self.height {
                    continue;
                }

                let target_index = (target_y * self.width + target_x) as usize;

                self.pixels[target_index] = blend_color(color, coverage);
            }
        }
    }

    pub fn draw_terminal(&mut self, terminal: &Terminal) {
        let cell_width = 12;
        let cell_height = 22;
        let padding = 20;

        for y in 0..terminal.height() {
            for x in 0..terminal.width() {
                let cell = terminal.cell(x,y);
                let character = cell.character();

                if character == ' ' {
                    continue;
                }

                let pixel_x = padding + x as u32 * cell_width;

                let pixel_y = padding + y as u32 * cell_height;

                self.draw_char(
                    character,
                    pixel_x,
                    pixel_y,
                    19.0,
                    0xE8E8F0,
                )
            }
        }
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }
}

fn blend_color(color: u32, coverage: u8) -> u32 {
    let amount = coverage as u32;

    let red = ((color >> 16) & 0xFF) * amount / 255;
    let green = ((color >> 8) & 0xFF) * amount / 255;
    let blue = (color & 0xFF) * amount / 255;

    (red << 16) | (green << 8) | blue
}