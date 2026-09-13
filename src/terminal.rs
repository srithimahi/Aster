#[derive(Clone, Debug)]
pub struct Cell {
    character: char,
    bold: bool,
    underline: bool,
}

impl Cell {
    pub fn character(&self) -> char {
        self.character
    }

    fn empty() -> Self {
        Self {
            character: ' ',
            bold: false,
            underline: false,
        }
    }
}

#[derive(Debug)]
pub struct Cursor {
    x: usize,
    y: usize,
}

impl Cursor {
    fn new() -> Self {
        Self {
            x: 0,
            y: 0,
        }
    }
}

pub struct Terminal {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    cursor: Cursor,
}

impl Terminal {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cell(&self, x: usize, y: usize) -> &Cell {
        let index = self.index(x, y);
        &self.cells[index]
    }

    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![Cell::empty(); width * height];

        Self {
            width,
            height,
            cells,
            cursor: Cursor::new(),
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn backspace(&mut self) {
        if self.cursor.x > 0 {
            self.cursor.x -= 1;
        }
    }

    pub fn cursor_x(&self) -> usize {
        self.cursor.x
    }

    pub fn cursor_y(&self) -> usize {
        self.cursor.y
    }

    fn put_character(&mut self, x: usize, y: usize, character: char) {
        let index = self.index(x, y);
        self.cells[index].character = character;
    }

    pub fn write_char(&mut self, character: char) {
        if character == '\n' {
            self.newline();
            return;
        }
    
        if self.cursor.x >= self.width {
            self.newline();
        }

        self.put_character(
            self.cursor.x,
            self.cursor.y,
            character,
        );

        self.cursor.x += 1;
    }

    pub fn write(&mut self, text: &str) {
        for character in text.chars() {
            self.write_char(character);
        }
    }

    fn newline(&mut self) {
        self.cursor.x = 0;
        self.cursor.y += 1;

        if self.cursor.y >= self.height {
            self.scroll_up();
            self.cursor.y = self.height - 1;
        }
    }

    fn scroll_up(&mut self) {
        for y in 1..self.height {
            for x in 0..self.width {
                let source_index = self.index(x, y);
                let destination_index = self.index(x, y - 1);

                self.cells[destination_index] = self.cells[source_index].clone();
            }
        }

        let last_row = self.height - 1;

        for x in 0..self.width {
            let index = self.index(x, last_row);
            self.cells[index] = Cell::empty();
        }
    }

    fn print_screen(&self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y);
                print!("{}", self.cells[index].character);
            }

            println!();
        }
    }

    pub fn carriage_return(&mut self) {
        self.cursor.x = 0;
    }

    pub fn line_feed(&mut self) {
        self.cursor.y += 1;

        if self.cursor.y >= self.height {
            self.scroll_up();
            self.cursor.y = self.height - 1;
        }
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor.x = x.min(self.width - 1);
        self.cursor.y = y.min(self.height - 1);
    }

    pub fn move_cursor_up(&mut self, amount: usize) {
        self.cursor.y = self.cursor.y.saturating_sub(amount);
    }

    pub fn move_cursor_down(&mut self, amount: usize) {
        self.cursor.y = (self.cursor.y + amount).min(self.height - 1);
    }

    pub fn move_cursor_left(&mut self, amount: usize) {
        self.cursor.x = self.cursor.x.saturating_sub(amount);
    }

    pub fn move_cursor_right(&mut self, amount: usize) {
        self.cursor.x = (self.cursor.x + amount).min(self.width - 1);
    }

    pub fn erase_line(&mut self, mode: usize) {
        match mode {
            0 => {
                for x in self.cursor.x..self.width {
                    let index = self.index(x, self.cursor.y);
                    self.cells[index] = Cell::empty();
                }
            }

            1 => {
                for x in 0..=self.cursor.x {
                    let index = self.index(x, self.cursor.y);
                    self.cells[index] = Cell::empty();
                }
            }

            2 => {
                for x in 0..self.width {
                    let index = self.index(x, self.cursor.y);
                    self.cells[index] = Cell::empty();
                }
            }

            _ => {}
        }
    }

    pub fn erase_display(&mut self, mode: usize) {
        match mode {
            0 => {
                self.erase_line(0);

                for y in (self.cursor.y + 1)..self.height {
                    for x in 0..self.width {
                        let index = self.index(x, y);
                        self.cells[index] = Cell::empty();
                    }
                }
            }

            1 => {
                for y in 0..self.cursor.y{
                    for x in 0..self.width {
                        let index = self.index(x, y);
                        self.cells[index] = Cell::empty();
                    }
                }

                self.erase_line(1);
            }

            2 => {
                for cell in &mut self.cells {
                    *cell = Cell::empty();
                }
            }

            _ => {}
        }
    }
}