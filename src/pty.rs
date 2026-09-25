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

fn debug_pty_output(bytes: &[u8]) {
    let mut readable = String::new();

    for &byte in bytes {
        match byte {
            0x1B => readable.push_str("<ESC>"),
            b'\r' => readable.push_str("<CR>"),
            b'\n' => readable.push_str("<LF>\n"),
            b'\t' => readable.push_str("<TAB>"),
            0x08 => readable.push_str("<BS>"),
            0x07 => readable.push_str("<BEL>"),

            0x20..=0x7E => {
                readable.push(byte as char);
            }

            _ => {
                readable.push_str(
                    &format!("<0x{byte:02X}>")
                );
            }
        }
    }

    println!("\n========== PTY OUTPUT ==========");
    println!("{readable}");
    println!("================================\n");
}

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
                        let bytes = &buffer[..bytes_read];
                        debug_pty_output(bytes);

                        let output = String::from_utf8_lossy(bytes).to_string();
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

    pub fn resize(
        &self,
        columns: usize,
        rows: usize,
    ) {
        if columns == 0 || rows == 0 {
            return;
        }

        let size = PtySize {
            rows: rows.min(u16::MAX as usize) as u16,
            cols: columns.min(u16::MAX as usize) as u16,
            pixel_width: 0,
            pixel_height: 0,
        };

        self.master
            .resize(size)
            .expect("Failed to resize man");
    }
}