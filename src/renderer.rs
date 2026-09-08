pub struct Renderer {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl Renderer {
    pub fn  new(width: u32, height: u32) -> Self {
        let pixels = vec![0; (width * height) as usize];

        Self {
            width,
            height,
            pixels,
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

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }
}