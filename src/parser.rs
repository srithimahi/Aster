use crate::terminal::Terminal;

#[derive(Debug)]
enum ParserState {
    Ground, 
    Escape,
    Csi,
}

pub struct AnsiParser {
    state: ParserState,
    parameters: String,
}

impl AnsiParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            parameters: String::new(),
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

        let response = match byte {
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

            b'H' | b'f' => None,

            b'J' => None,

            b'K' => None,

            b'h' | b'l' => None,

            _ => None,
        };

        self.parameters.clear();
        self.state = ParserState::Ground;

        response
    }
}

