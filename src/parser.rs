use crate::terminal::{Terminal, TerminalColor};

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

            b'm' => {
                Self::process_sgr(
                    &parameters,
                    terminal,
                );
                None
            }

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

    fn process_sgr(
        parameters: &[usize],
        terminal: &mut Terminal,
    ) {
        if parameters.is_empty() {
            terminal.reset_style();
            return;
        }

        let mut index = 0;

        while index < parameters.len() {
            let parameter = parameters[index];
            match parameter {
                0 => {
                    terminal.reset_style();
                }

                1 => {
                    terminal.set_bold(true);
                }

                4 => {
                    terminal.set_underline(true);
                }

                22 => {
                    terminal.set_bold(false);
                }

                24 => {
                    terminal.set_underline(false);
                }

                30..=37 => {
                    terminal.set_foreground(
                        TerminalColor::Indexed(
                            (parameter - 30) as u8
                        ),
                    );
                }

                39 => {
                    terminal.set_foreground(
                        TerminalColor::Default
                    );
                }

                40..=47 => {
                    terminal.set_background(
                        TerminalColor::Indexed(
                            (parameter - 40) as u8
                        ),
                    );
                }

                49 => {
                    terminal.set_background(
                        TerminalColor::Default
                    );
                }

                90..=97 => {
                    terminal.set_foreground(
                        TerminalColor::Indexed(
                            (parameter - 90 + 8) as u8
                        ),
                    );
                }

                100..=107 => {
                    terminal.set_background(
                        TerminalColor::Indexed(
                            (parameter - 100 + 8) as u8
                        ),
                    );
                }

                38 => {
                    index = Self::process_extended_color(
                        parameters,
                        index,
                        terminal,
                        true,
                    );
                }

                48 => {
                    index = Self::process_extended_color(
                        parameters,
                        index,
                        terminal,
                        false,
                    );
                }

                _ => {}
            }

            index += 1;
        }
    }

    fn process_extended_color(
        parameters: &[usize],
        index: usize,
        terminal: &mut Terminal,
        foreground: bool,
    ) -> usize {
        let Some(&color_type) = parameters.get(index + 1) else {
            return index;
        };

        match color_type {
            5 => {
                let Some(&color_index) = parameters.get(index + 2)
                else {
                    return index;
                };

                let color = TerminalColor::Indexed(color_index as u8);

                if foreground {
                    terminal.set_foreground(color);
                } else {
                    terminal.set_background(color);
                }

                index + 2
            }

            2 => {
                let Some(&red) = parameters.get(index + 2)
                else {
                    return index;
                };

                let Some(&green) = parameters.get(index + 3)
                else {
                    return index;
                };

                let Some(&blue) = parameters.get(index + 4)
                else {
                    return index;
                };

                let color = TerminalColor::Rgb(
                    red.min(255) as u8,
                    green.min(255) as u8,
                    blue.min(255) as u8,
                );

                if foreground {
                    terminal.set_foreground(color);
                } else {
                    terminal.set_background(color);
                }

                index + 4
            }

            _ => index,
        }
    }
}

