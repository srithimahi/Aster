use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver},
    thread,
};

use portable_pty::{
    native_pty_system,
    CommandBuilder,
    MasterPty,
    PtySize,
};

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    receiver: Receiver<String>,
}

impl PtySession {
    pub fn new() -> Self {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }).expect("Failed to create PTY");

        let command = CommandBuilder::new("powershell.exe");

        let _child = pair
            .slave
            .spawn_command(command)
            .expect("Failed to start PowerShell");

        drop(pair.slave);

        let writer = pair
            .master
            .take_writer()
            .expect("Failed to open PTY writer");

        let mut reader = pair
            .master 
            .try_clone_reader()
            .expect("Failed to open PTY reader");

        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let mut buffer = [0u8; 4096];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,

                    Ok(bytes_read) => {
                        let output =
                            String::from_utf8_lossy(
                                &buffer[..bytes_read]
                            )
                            .to_string();

                        if sender.send(output).is_err() {
                            break;
                        }
                    }

                    Err(error) => {
                        eprintln!("PTY read error: {error}");
                        break;
                    }
                }
            }
        });

        Self {
            master: pair.master,
            writer,
            receiver,
        }
    }

    pub fn try_read(&self) -> Option<String> {
        self.receiver.try_recv().ok()
    }

    pub fn write(&mut self, text: &str) {
        self.writer 
            .write_all(text.as_bytes())
            .expect("Failed to write to PTY");

        self.writer 
            .flush()
            .expect("Failed to flush PTY");
    }
}