use crate::terminal::Terminal;

#[derive(Debug)]
enum ParserState {
    Ground, 
    Escape,
    Csi,
    Osc,
}

pub struct AnsiParser {
    state: ParserState,
    parameters: String,
    osc_data: String,
}

impl AnsiParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            parameters: String::new(),
            osc_data: String::new(),
        }
    }

    fn parsed_parameters(&self) -> Vec<usize> {
        if self.parameters.is_empty() {
            return Vec::new();
        }

        self.parameters 
            .split(';')
            .map(|part| {
                part.parse::<usize>().unwrap_or(0)
            })
            .collect()
    }

    fn parameter_or(
        parameters:&[usize],
        index: usize,
        default: usize,
    ) -> usize {
        match parameters.get(index) {
            Some(&0) | None => default,
            Some(&value) => value,
        }
    }

    pub fn process_byte(
        &mut self,
        byte: u8,
        terminal: &mut Terminal,
    ) -> Option<String> {
        match self.state {
            ParserState::Ground => {
                self.process_ground(byte, terminal);
                None
            }

            ParserState::Escape => {
                self.process_escape(byte);
                None
            }

            ParserState::Csi => {
                self.process_csi(byte, terminal)
            }

            ParserState::Osc => {
                self.process_osc(byte);
                None
            }
        }
    }

    fn process_ground(
        &mut self,
        byte: u8,
        terminal: &mut Terminal,
    ) {
        match byte {
            0x1B => {
                self.state = ParserState::Escape;
            }

            b'\r' => {
                
                terminal.carriage_return();
            }

            b'\n' => {
                terminal.line_feed();
            }

            0x08 => {
                terminal.backspace();
            }

            byte if byte >= 0x20 => {
                let character = byte as char;
                terminal.write_char(character);
            }

            _ => {}
        }
    }

    fn process_escape(&mut self, byte: u8) {
        match byte {
            b'[' => {
                self.parameters.clear();
                self.state = ParserState::Csi;
            }

            b']' => {
                self.osc_data.clear();
                self.state = ParserState::Osc;
            }

            _ => {
                self.state = ParserState::Ground;
            }
        }
    }

    fn process_csi(
        &mut self,
        byte: u8,
        terminal: &mut Terminal,
    ) -> Option<String> {
        if (0x30..=0x3F).contains(&byte) {
            self.parameters.push(byte as char);
            return None;
        }

        let parameters = self.parsed_parameters();

        let response = match byte {
            b'A' => {
                let amount = 
                    Self::parameter_or(&parameters, 0, 1);

                terminal.move_cursor_up(amount);
                None
            }

            b'B' => {
                let amount =
                    Self::parameter_or(&parameters, 0, 1);

                terminal.move_cursor_down(amount);
                None
            }

            b'C' => {
                let amount =
                    Self::parameter_or(&parameters, 0, 1);
                
                terminal.move_cursor_right(amount);
                None
            }

            b'D' => {
                let amount = 
                    Self::parameter_or(&parameters, 0, 1);
                
                terminal.move_cursor_left(amount);
                None
            }

            b'H' | b'f' => {
                let row =
                    Self::parameter_or(&parameters, 0, 1);

                let column =
                    Self::parameter_or(&parameters, 1, 1);

                terminal.set_cursor(
                    column.saturating_sub(1),
                    row.saturating_sub(1),
                );

                None
            }

            b'n' if self.parameters == "6" => {
                let row = terminal.cursor_y() + 1;
                let column = terminal.cursor_x() + 1;

                Some(format!(
                    "\x1b[{};{}R",
                    row,
                    column,
                ))
            }

            b'm' => None,
            b'J' => {
                let mode = Self::parameter_or(&parameters, 0, 0);
                terminal.erase_display(mode);
                None
            }

            b'K' => {
                let mode = Self::parameter_or(&parameters, 0, 0);
                terminal.erase_line(mode);
                None
            }

            b'h' | b'l' => None,

            _ => None,
        };

        self.parameters.clear();
        self.state = ParserState::Ground;

        response
    }

    fn process_osc(&mut self, byte: u8) {
        match byte {
            0x07 => {
                println!("OSC: {}", self.osc_data);
                self.osc_data.clear();
                self.state = ParserState::Ground;
            }

            _ => {
                self.osc_data.push(byte as char);
            } 
        }
    }
}

