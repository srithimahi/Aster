mod parser;
mod pty;
mod renderer;
mod terminal;

use parser::AnsiParser;
use pty::PtySession;

use std::{
    num::NonZeroU32,
    sync::Arc,
};

use softbuffer::{Context, Surface};

use renderer::Renderer;
use terminal::Terminal;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

struct AsterApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    terminal: Terminal,
    renderer: Renderer,
    pty: PtySession,
    parser: AnsiParser,
}

impl AsterApp {
    fn new() -> Self {
        let terminal = Terminal::new(80,24);

        Self {
            window: None,
            surface: None,
            terminal,
            renderer: Renderer::new(900, 600),
            pty: PtySession::new(),
            parser: AnsiParser::new(),
        }
    }
}

impl ApplicationHandler for AsterApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);

        let attributes = Window::default_attributes()
            .with_title("Aster")
            .with_inner_size(LogicalSize::new(900.0, 600.0));

        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("Failed to create Aster window"),
        );

        let context = Context::new(window.clone())
            .expect("Failed to create graphics context");

        let surface = Surface::new(&context, window.clone())
            .expect("Failed to create graphics surface");

        self.surface = Some(surface);
        self.window = Some(window.clone());

        window.request_redraw();

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn about_to_wait(
        &mut self,
        _event_loop: &ActiveEventLoop,
    ) {
        let mut received_output = false;

        while let Some(output) = self.pty.try_read() {
            for byte in output.bytes() {
                if let Some(response) =
                    self.parser.process_byte(
                        byte,
                        &mut self.terminal,
                    )
                {
                    self.pty.write(&response);
                }
            }

            received_output = true;
        }

        if received_output {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);

                println!(
                    "Aster resized: {} x {}", size.width, size.height
                );

                window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let Some(surface) = &mut self.surface else {
                    return;
                };

                let size = window.inner_size();

                let Some(width) = NonZeroU32::new(size.width) else {
                    return;
                };

                let Some(height) = NonZeroU32::new(size.height) else {
                    return;
                };

                surface
                    .resize(width, height)
                    .expect("Failed to resize surface");

                self.renderer.clear(0x101218);

                self.renderer.draw_terminal(&self.terminal);

                let mut buffer = surface
                    .buffer_mut()
                    .expect("Failed to access window buffer");

                buffer.copy_from_slice(self.renderer.pixels());

                window.pre_present_notify();

                buffer  
                    .present()
                    .expect("Failed to present frame")
            }

            WindowEvent::MouseWheel { delta, ..} => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        y.round() as i32
                    }

                    MouseScrollDelta::PixelDelta(position) => {
                        (position.y / 18.0).round() as i32
                    }
                };

                if lines > 0 {
                    self.terminal.scroll_view_up(lines as usize);
                } else if lines < 0 {
                    self.terminal.scroll_view_down((-lines) as usize);
                }

                window.request_redraw();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                match &event.logical_key {
                    Key::Named(NamedKey::Enter) => {
                        self.pty.write("\r");
                    }

                    Key::Named(NamedKey::Backspace) => {
                        self.pty.write("\u{8}");
                    }

                    _ => {
                        if let Some(text) = &event.text {
                            self.pty.write(text);
                        }
                    }
                }

                window.request_redraw();
            }

            _ => {}
        }
    }
}

fn main() {
    let event_loop = 
        EventLoop::new().expect("Failed to create the event loop");

    let mut app = AsterApp::new();

    event_loop
        .run_app(&mut app)
        .expect("Aster crashed");
}