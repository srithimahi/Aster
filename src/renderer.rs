use crate::terminal::{Terminal, TerminalColor};
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

        let baseline_y = y as i32 + size as i32;

        for glyph_y in 0..metrics.height {
            for glyph_x in 0..metrics.width {
                let source_index = 
                    glyph_y * metrics.width + glyph_x;

                let coverage = bitmap[source_index];

                if coverage == 0 {
                    continue;
                }

                let target_x = x as i32 + metrics.xmin + glyph_x as i32;
                let target_y = baseline_y - metrics.ymin - metrics.height as i32 + glyph_y as i32;

                if target_x < 0 || target_y < 0 {
                    continue;
                }

                let target_x = target_x as u32;
                let target_y = target_y as u32;

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
        let cell_height = 18;
        let padding = 20;

        for y in 0..terminal.height() {
            for x in 0..terminal.width() {
                let cell = terminal.visible_cell(x,y);
                let character = cell.character();

                let pixel_x = padding + x as u32 * cell_width;
                let pixel_y = padding + y as u32 * cell_height;

                let background = terminal_color(cell.background());

                if !matches!(
                    cell.background(),
                    TerminalColor::Default
                ) {
                    let background = terminal_color(cell.background());
                    self.draw_rect(
                        pixel_x,
                        pixel_y,
                        cell_width,
                        cell_height,
                        background,
                    );
                }

                if character == ' ' {
                    continue;
                }

                let foreground = terminal_color(cell.foreground());

                self.draw_char(
                    character,
                    pixel_x,
                    pixel_y,
                    19.0,
                    foreground,
                )
            }
        }

        if terminal.viewport_offset() == 0 {
            let cursor_x =
                padding + terminal.cursor_x() as u32 * cell_width;

            let cursor_y =
                padding + terminal.cursor_y() as u32 * cell_height;

            self.draw_rect(
                cursor_x,
                cursor_y + cell_height - 2,
                cell_width,
                2,
                0xE8E8F0,
            );
        }
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }
}

fn terminal_color(color: &TerminalColor) -> u32 {
    match color {
        TerminalColor::Default => 0xE8E8F0,

        TerminalColor::Indexed(index) => {
            indexed_color(*index)
        }

        TerminalColor::Rgb(red, green, blue) => {
            ((*red as u32) << 16) | ((*green as u32) << 8) | (*blue as u32)
        }
    }
}

fn blend_color(color: u32, coverage: u8) -> u32 {
    let amount = coverage as u32;

    let red = ((color >> 16) & 0xFF) * amount / 255;
    let green = ((color >> 8) & 0xFF) * amount / 255;
    let blue = (color & 0xFF) * amount / 255;

    (red << 16) | (green << 8) | blue
}

fn indexed_color(index: u8) -> u32 {
    match index {
        0 => 0x000000,
        1 => 0xCC5555,
        2 => 0x55CC55,
        3 => 0xCCCC55,
        4 => 0x5555CC,
        5 => 0xCC55CC,
        6 => 0x55CCCC,
        7 => 0xCCCCCC,

        8 => 0x555555,
        9 => 0xFF7777,
        10 => 0x77FF77,
        11 => 0xFFFF77,
        12 => 0x7777FF,
        13 => 0xFF77FF,
        14 => 0x77FFFF,
        15 => 0xFFFFFF,

        16..=231 => {
            let cube_index = index - 16;

            let red = cube_index / 36;
            let green = (cube_index % 36) / 6;
            let blue = cube_index % 6;

            let level = |value: u8| -> u32 {
                if value == 0 {
                    0
                } else {
                    55 + 40 * value as u32
                }
            };

            let red = level(red);
            let green = level(green);
            let blue = level(blue);

            (red << 16) | (green << 8) | blue
        }

        232..=255 => {
            let gray = 8 + 10 * (index as u32 - 232);
            (gray << 16) | (gray << 8) | gray
        }
    }
}