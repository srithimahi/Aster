use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};

use crate::{
    parser::{
        AnsiParser,
        ParserEvent,
    },
    pty::PtySession,
    terminal::Terminal,
};

static NEXT_PANE_ID: AtomicUsize = 
    AtomicUsize::new(1);

pub struct Pane {
    id: usize,
    title: String,
    terminal: Terminal,
    pty: PtySession,
    parser: AnsiParser,
}

impl Pane {
    pub fn new(
        columns: usize,
        rows: usize,
    ) -> Self {
            Self {
            id: NEXT_PANE_ID.fetch_add(
                1,
                Ordering::Relaxed,
            ),

            title: String::from(
                "PowerShell"
            ),

            terminal: Terminal::new(
                columns,
                rows,
            ),

            pty: PtySession::new(),
            parser: AnsiParser::new(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn title(
        &self,
    ) -> &str {
        &self.title
    }

    fn handle_parser_event(
        &mut self,
        event: ParserEvent,
    ) {
        match event {
            ParserEvent::SetTitle(
                title
            ) => {
                self.title = title;
                println!(
                    "Pane {} title changed to: {}",
                    self.id,
                    self.title,
                );
            }
        }
    }

    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    pub fn terminal_mut(
        &mut self,
    ) -> &mut Terminal {
        &mut self.terminal
    }

    pub fn write(
        &mut self,
        text: &str,
    ) {
        self.pty.write(text);
    }

    pub fn resize(
        &mut self,
        columns: usize,
        rows: usize,
    ) {
        self.terminal.resize(
            columns,
            rows,
        );

        self.pty.resize(
            columns,
            rows,
        );
    }

    pub fn process_output(
        &mut self,
    ) -> bool {
        let mut received_output = false;
        while let Some(output) = self.pty.try_read() {
            for byte in output.bytes() {
                let parser_output = self.parser.process_byte(
                    byte,
                    &mut self.terminal,
                );

                if let Some(response) = parser_output.response {
                    self.pty.write(&response);
                }

                if let Some(event) = parser_output.event {
                    self.handle_parser_event(event);
                }
            }
            received_output = true;
        }
        received_output
    }
}
