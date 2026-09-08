use std::{
    num::NonZeroU32,
    sync::Arc,
};

use softbuffer::{Context, Surface};

mod renderer;
mod terminal;

use renderer::Renderer;
use terminal::Terminal;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct AsterApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    terminal: Terminal,
    renderer: Renderer,
}

impl AsterApp {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            terminal: Terminal::new(80, 24),
            renderer: Renderer::new(900, 600),
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

                self.renderer.draw_rect(
                    20, 20, 300, 50, 0x8A7CF0,
                );

                let mut buffer = surface
                    .buffer_mut()
                    .expect("Failed to access window buffer");

                buffer.copy_from_slice(self.renderer.pixels());

                window.pre_present_notify();

                buffer  
                    .present()
                    .expect("Failed to present frame")
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