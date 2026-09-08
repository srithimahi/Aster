#[derive(Clone, Debug)]
pub struct Cell {
    character: char,
    bold: bool,
    underline: bool,
}

impl Cell {
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

    fn put_character(&mut self, x: usize, y: usize, character: char) {
        let index = self.index(x, y);
        self.cells[index].character = character;
    }

    fn write_char(&mut self, character: char) {
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

    fn write(&mut self, text: &str) {
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
}