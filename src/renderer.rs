use crate::terminal::{Terminal, TerminalColor};
use crate::theme::ThemePalette;
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

                let background = self.pixels[target_index];
                self.pixels[target_index] = blend_color(background, color, coverage);
            }
        }
    }

    pub fn draw_terminal(
        &mut self,
        terminal: &Terminal,
        origin_x: u32,
        origin_y: u32,
        cell_width: u32,
        cell_height: u32,
        font_size: f32,
        default_foreground: u32,
        default_background: u32,
        cursor_color: u32,
        palette: &ThemePalette,
    ) {
        for y in 0..terminal.height() {
            for x in 0..terminal.width() {
                let cell = terminal.visible_cell(x,y);
                let character = cell.character();

                let pixel_x = origin_x + x as u32 * cell_width;
                let pixel_y = origin_y + y as u32 * cell_height;

                let background = terminal_color(
                    cell.background(),
                    default_background,
                    palette,
                );

                let is_selected =
                    terminal.is_cell_selected(x, y);

                let cell_background =
                    if is_selected {
                        cursor_color
                    } else {
                        background
                    };

                if is_selected
                    || !matches!(
                        cell.background(),
                        TerminalColor::Default
                    )
                {
                    self.draw_rect(
                        pixel_x,
                        pixel_y,
                        cell_width,
                        cell_height,
                        cell_background,
                    );
                }

                if character == ' ' {
                    continue;
                }

                let foreground = terminal_color(
                    cell.foreground(),
                    default_foreground,
                    palette,
                );

                self.draw_char(
                    character,
                    pixel_x,
                    pixel_y,
                    font_size,
                    foreground,
                );
            }
        }

        if terminal.viewport_offset() == 0 {
            let cursor_x = origin_x + terminal.cursor_x() as u32 * cell_width;
            let cursor_y = origin_y + terminal.cursor_y() as u32 * cell_height;

            self.draw_rect(
                cursor_x,
                cursor_y + cell_height - 2,
                cell_width,
                2,
                cursor_color,
            );
        }
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        x: u32,
        y: u32,
        font_size: f32,
        color: u32,
    ) {
        let mut current_x = x;
        for character in text.chars() {
            self.draw_char(
                character,
                current_x,
                y,
                font_size,
                color,
            );
            current_x += font_size as u32;
        }
    }

    pub fn draw_tab_bar(
        &mut self,
        tab_count: usize,
        active_tab: usize,
        height: u32,
        font_size: f32,
        foreground: u32,
        background: u32,
        active_background: u32,
    ) {
        self.draw_rect(
            0,
            0,
            self.width,
            height,
            background,
        );

        let tab_width = 140;
        for index in 0..tab_count {
            let x = index as u32 * tab_width;
            if index == active_tab {
                self.draw_rect(
                    x,
                    0,
                    tab_width,
                    height,
                    active_background,
                );
            }

            let label = format!("Tab {}", index + 1);
            self.draw_text(
                &label,
                x + 12,
                5,
                font_size,
                foreground,
            );
        }
    }

    pub fn draw_pane_border(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: u32,
    ) {
        if width < 2 || height < 2 {
            return;
        }

        let thickness = 2;

        self.draw_rect(
            x,
            y,
            width,
            thickness,
            color,
        );

        self.draw_rect(
            x,
            y + height - thickness,
            width,
            thickness,
            color,
        );

        self.draw_rect(
            x,
            y,
            thickness,
            height,
            color,
        );

        self.draw_rect(
            x + width - thickness,
            y,
            thickness,
            height,
            color,
        );
    }

    pub fn draw_pane_separator(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: u32,
    ) {
        if width == 0 || height == 0 {
            return;
        }

        self.draw_rect(
            x,
            y,
            width,
            height,
            color,
        );
    }
}

fn terminal_color(
    color: &TerminalColor,
    default_color: u32,
    palette: &ThemePalette,
) -> u32 {
    match color {
        TerminalColor::Default => default_color,

        TerminalColor::Indexed(index) => {
            indexed_color(*index, palette)
        }

        

        TerminalColor::Rgb(red, green, blue) => {
            ((*red as u32) << 16) | ((*green as u32) << 8) | (*blue as u32)
        }
    }
}

 fn blend_color(
    background: u32,
    foreground: u32,
    coverage: u8,
 ) -> u32 {
    let alpha = coverage as u32;
    let inverse_alpha = 255 - alpha;

    let background_red = (background >> 16) & 0xFF;
    let background_green = (background >> 8) & 0xFF;
    let background_blue = background & 0xFF;

    let foreground_red = (foreground >> 16) & 0xFF;
    let foreground_green = (foreground >> 8) & 0xFF;
    let foreground_blue = (foreground) & 0xFF;

    let red = (foreground_red * alpha + background_red * inverse_alpha) / 255;
    let green = (foreground_green * alpha + background_green * inverse_alpha) / 255;
    let blue = (foreground_blue * alpha + background_blue * inverse_alpha) / 255;
    
    (red << 16) | (green << 8) | blue
 }

fn indexed_color(index: u8, palette: &ThemePalette,) -> u32 {
    match index {
        0..=15 => {
            palette.color(index)
        }

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