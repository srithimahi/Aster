#[derive(Clone, Debug)]
pub struct Cell {
    character: char,
    foreground: TerminalColor,
    background: TerminalColor,
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
            foreground: TerminalColor::Default,
            background: TerminalColor::Default,
            bold: false,
            underline: false,
        }
    }

    pub fn foreground(&self) -> &TerminalColor {
        &self.foreground
    }

    pub fn background(&self) -> &TerminalColor {
        &self.background
    }
}

#[derive(Debug)]
pub struct Cursor {
    x: usize,
    y: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SelectionPoint {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub start: SelectionPoint,
    pub end: SelectionPoint,
}

#[derive(Clone, Copy, Debug)]
pub struct SearchMatch {
    pub start: SelectionPoint,
    pub end: SelectionPoint,
}

impl Cursor {
    fn new() -> Self {
        Self {
            x: 0,
            y: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TerminalColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Debug)]
pub struct TextStyle {
    foreground: TerminalColor,
    background: TerminalColor,
    bold: bool,
    underline: bool,
}

impl TextStyle {
    fn new() -> Self {
        Self {
            foreground: TerminalColor::Default,
            background: TerminalColor::Default,
            bold: false,
            underline: false,
        }
    }
}

pub struct Terminal {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    cursor: Cursor,
    scrollback: Vec<Vec<Cell>>,
    viewport_offset: usize,
    current_style: TextStyle,
    selection: Option<Selection>,
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
            scrollback: Vec::new(),
            viewport_offset: 0,
            current_style: TextStyle::new(),
            selection: None,
        }
    }

    pub fn scroll_view_up(&mut self, amount: usize) {
        self.viewport_offset = (self.viewport_offset + amount)
            .min(self.scrollback.len());
    }

    pub fn scroll_view_down(&mut self, amount: usize) {
        self.viewport_offset =
            self.viewport_offset.saturating_sub(amount);
    }

    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
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
        self.cells[index].foreground = self.current_style.foreground.clone();
        self.cells[index].background = self.current_style.background.clone();
        self.cells[index].bold = self.current_style.bold;
        self.cells[index].underline = self.current_style.underline;
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
        let mut old_row = Vec::with_capacity(self.width);

        for x in 0..self.width {
            let index = self.index(x, 0);
            old_row.push(self.cells[index].clone());
        }

        self.scrollback.push(old_row);

        for y in 1..self.height {
            for x in 0..self.width {
                let source_index = self.index(x, y);
                let destination_index = self.index(x, y-1);

                self.cells[destination_index] = self.cells[source_index].clone();
            }
        }

        let last_row = self.height - 1;

        for x in 0..self.width {
            let index = self.index(x, last_row);
            self.cells[index] = Cell::empty();
        }
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
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

    pub fn visible_row_to_history_row(
        &self,
        y: usize,
    ) -> usize {
        let viewport_start = self.scrollback
        .len()
        .saturating_sub(
            self.viewport_offset
        );

        viewport_start + y
    }

    fn history_cell(
        &self,
        x: usize,
        history_y: usize,
    ) -> &Cell {
        let history_length = self.scrollback.len();

        if history_y < history_length {
            &self.scrollback[history_y][x]
        } else {
            let screen_y = history_y - history_length;
            self.cell(x, screen_y,)
        }
    }

    pub fn visible_cell(&self, x: usize, y: usize) -> &Cell {
        if self.viewport_offset == 0 {
            return self.cell(x,y);
        }

        let history_length = self.scrollback.len();

        let viewport_start = history_length.saturating_sub(self.viewport_offset);

        let virtual_row = viewport_start + y;

        if virtual_row < history_length {
            &self.scrollback[virtual_row][x]
        } 
        else {
            let screen_row = virtual_row - history_length;
            self.cell(x, screen_row)
        }
    }

    pub fn reset_style(&mut self) {
        self.current_style = TextStyle::new();
    }

    pub fn set_bold(&mut self, enabled: bool) {
        self.current_style.bold = enabled;
    }

    pub fn set_underline(&mut self, enabled: bool) {
        self.current_style.underline = enabled;
    }

    pub fn set_foreground(&mut self, color: TerminalColor) {
        self.current_style.foreground = color;
    }

    pub fn set_background(&mut self, color: TerminalColor) {
        self.current_style.background = color;
    }

    pub fn resize(
        &mut self,
        new_width: usize,
        new_height: usize,
    ) {
        if new_width == 0 || new_height == 0 {
            return;
        }

        if new_width == self.width && new_height == self.height {
            return;
        }

        for row in &mut self.scrollback {
            row.resize(
                new_width,
                Cell::empty(),
            );
        }

        let mut new_cells = vec![Cell::empty(); new_width * new_height];
        let copy_width = self.width.min(new_width);
        let copy_height = self.height.min(new_height);

        for y in 0..copy_height {
            for x in 0..copy_width {
                let old_index = y * self.width + x;
                let new_index = y * new_width + x;

                new_cells[new_index] = self.cells[old_index].clone();
            }
        }

        self.cells = new_cells;
        self.width = new_width;
        self.height = new_height;

        self.cursor.x = self.cursor.x.min(new_width - 1);
        self.cursor.y = self.cursor.y.min(new_height - 1);

        self.viewport_offset = self.viewport_offset.min(
            self.scrollback.len()
        );
    }

    pub fn start_selection(
        &mut self,
        x: usize,
        y: usize,
    ) {
        let x = x.min(self.width - 1);
        let visible_y = y.min(self.height - 1);

        let history_y = self.visible_row_to_history_row(visible_y);
        self.selection = Some(
            Selection {
                start: SelectionPoint {
                    x,
                    y: history_y,
                },
                end: SelectionPoint {
                    x,
                    y: history_y,
                },
            }
        );
    }

    pub fn update_selection(
        &mut self,
        x: usize,
        y: usize,
    ) {
        let x = x.min(self.width - 1);
        let visible_y = y.min(self.height - 1);
        let history_y = self.visible_row_to_history_row(
            visible_y
        );

        if let Some(selection) = &mut self.selection {
            selection.end = SelectionPoint {
                x,
                y: history_y,
            };
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn selection(
        &self,
    ) -> Option<Selection> {
        self.selection
    }

    pub fn is_cell_selected(
        &self,
        x: usize,
        y: usize, 
    ) -> bool {
        let Some(selection) = self.selection
        else {
            return false;
        };

        let history_y = self.visible_row_to_history_row(y);
        let start_index = selection.start.y * self.width + selection.start.x;
        let end_index = selection.end.y * self.width + selection.end.x;

        let cell_index = history_y * self.width + x;

        let (first, last) = 
            if start_index <= end_index {
                (start_index, end_index)
            } else {
                (end_index, start_index)
            };

        cell_index >= first && cell_index <= last
    }

    pub fn selected_text(&self) -> Option<String> {
        let selection = self.selection?;
        let start_index = selection.start.y * self.width + selection.start.x;
        let end_index = selection.end.y * self.width + selection.end.x;

        let(first, last) =
            if start_index <= end_index {
                (start_index, end_index)
            } else {
                (end_index, start_index)
            };

        let first_y = first / self.width;
        let last_y = last / self.width;

        let mut result = String::new();

        for y in first_y..=last_y {
            let row_start_x = if y == first_y {
                first % self.width 
            } else {
                0
            };

            let row_end_x =
                if y == last_y {
                    last % self.width
                } else {
                    self.width - 1
                };

            let mut line = String::new();

            for x in row_start_x..=row_end_x {
                line.push(
                    self.history_cell(x, y).character()
                );
            }

            while line.ends_with(' ') {
                line.pop();
            }

            result.push_str(&line);
            if y != last_y {
                result.push('\n');
            }
        }
        Some(result)
    }

    fn history_row_text(
        &self,
        history_y: usize,
    ) -> String {
        let mut text = String::with_capacity(self.width);

        for x in 0..self.width {
            text.push(
                self.history_cell(
                    x,
                    history_y,
                ).character()
            );
        }
        text
    }

    pub fn search(
        &self,
        query: &str,
    ) -> Vec<SearchMatch> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_chars: Vec<char> = query
            .to_lowercase()
            .chars()
            .collect();

        if query_chars.is_empty() {
            return Vec::new();
        }

        let total_rows = self.scrollback.len() + self.height;
        let mut matches = Vec::new();
        for history_y in 0..total_rows {
            let row = self.history_row_text(history_y);
            let row_chars: Vec<char> = row
                .to_lowercase()
                .chars()
                .collect();
            
            if query_chars.len() > row_chars.len() {
                continue;
            }

            let last_start = row_chars.len() - query_chars.len();

            for start_x in 0..=last_start {
                let end = start_x + query_chars.len();
                if row_chars[start_x..end] == query_chars[..] {
                    matches.push(
                        SearchMatch {
                            start: SelectionPoint {
                                x: start_x,
                                y: history_y,
                            },

                            end: SelectionPoint {
                                x: end - 1,
                                y: history_y,
                            },
                        }
                    );
                }
            }
        }

        matches
    }

    pub fn is_cell_in_search_match(
        &self,
        x: usize,
        y: usize,
        search_match: &SearchMatch,
    ) -> bool {
        let history_y = self.visible_row_to_history_row(y);

        if history_y < search_match.start.y || history_y > search_match.end.y {
            return false;
        }

        if search_match.start.y == search_match.end.y {
            return history_y == search_match.start.y 
            && x >= search_match.start.x
            && x <= search_match.end.x;
        }

        if history_y == search_match.start.y {
            return x >= search_match.start.x;
        }

        if history_y == search_match.end.y {
            return x <= search_match.end.x;
        }

        true
    }

    pub fn reveal_history_row(
        &mut self,
        history_y: usize,
    ) {
        let history_length = self.scrollback.len();
        let total_rows = history_length + self.height;

        if total_rows == 0 {
            return;
        }

        let history_y = history_y.min(total_rows - 1);
        let current_start = history_length.saturating_sub(
            self.viewport_offset
        );

        let current_end = current_start + self.height.saturating_sub(1);
        if history_y >= current_start && history_y <= current_end {
            return;
        }

        let desired_screen_row = self.height / 2;
        let desired_start = history_y.saturating_sub(
            desired_screen_row
        );

        self.viewport_offset = history_length.saturating_sub(
            desired_start
        ).min(history_length);
    }
}