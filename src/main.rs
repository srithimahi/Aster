mod terminal;

use terminal::Terminal;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct AsterApp {
    window: Option<Window>,
    terminal: Terminal,
}

impl AsterApp {
    fn new() -> Self {
        Self {
            window: None,
            terminal: Terminal::new(80, 24),
        }
    }
}

impl ApplicationHandler for AsterApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Aster")
            .with_inner_size(LogicalSize::new(900.0, 600.0));

        let window = event_loop
            .create_window(attributes)
            .expect("Failed to create Aster window");

        self.window = Some(window);
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
                println!("Aster resized: {} x {}", size.width, size.height);
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