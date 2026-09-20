use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};

use crate::{
    parser::AnsiParser,
    pty::PtySession,
    terminal::Terminal,
};

static NEXT_PANE_ID: AtomicUsize = 
    AtomicUsize::new(1);

pub struct Pane {
    id: usize,
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

        while let Some(output) =
            self.pty.try_read()
        {

            for byte in output.bytes() {
                if let Some(response) =
                    self.parser.process_byte(
                        byte,
                        &mut self.terminal,
                    )
                {
                    self.pty.write(
                        &response
                    );
                }
            }

            received_output = true;
        }

        received_output
    }
}
