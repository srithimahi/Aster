mod config;
mod parser;
mod pty;
mod renderer;
mod terminal;
mod theme;
mod mux;

use arboard::Clipboard;
use config::{Config, parse_color};
use theme::Theme;
use mux::tab::Tab;
use mux::layout::{
    FocusDirection,
    SplitDirection,
};

use std::{
    num::NonZeroU32,
    sync::Arc,
};

use softbuffer::{Context, Surface};

use renderer::Renderer;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent, },
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey,},
    window::{Window, WindowId},
};

const TAB_BAR_HEIGHT: u32 = 30;

#[derive(Clone, Copy, Debug)]
struct SplitDrag {
    split_id: usize,
}

struct AsterApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    tabs: Vec<Tab>,
    active_tab: usize,
    renderer: Renderer,
    config: Config,
    theme: Theme,
    modifiers: ModifiersState,
    dragging_split: Option<SplitDrag>,
    selecting_text: bool,

    mouse_x: f64,
    mouse_y: f64,
}

impl AsterApp {
    fn new(config: Config, theme:Theme) -> Self {
        Self {
            window: None,
            surface: None,
            tabs: vec![
                Tab::new(80, 24)
            ],
            active_tab: 0,
            renderer: Renderer::new(900, 600),
            config,
            theme,
            modifiers: ModifiersState::empty(),
            dragging_split: None,
            selecting_text: false,

            mouse_x: 0.0,
            mouse_y: 0.0, 
        }
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    fn new_tab(&mut self) {
        let columns = self.active_tab()
            .terminal()
            .width();

        let rows = self.active_tab()
            .terminal()
            .height();

        self.tabs.push(
            Tab::new(columns, rows)
        );

        self.active_tab = self.tabs.len() - 1;

        println!("Created the tabbie {}", self.active_tab + 1);
    }

    fn next_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }

        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        println!("Switched to tabbie {}", self.active_tab + 1);
    }

    fn previous_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        if self.active_tab == 0 {
            self.active_tab = self.tabs.len() - 1;
        } else {
            self.active_tab -= 1;
        }

        println!("Switched to tabbie {}", self.active_tab + 1);
    }

    fn close_active_tab(&mut self) {
        if self.tabs.len() <= 1 {
            println!("Cannor close the last tabbie");
            return;
        }

        let closed_tab = self.active_tab;
        self.tabs.remove(closed_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }

        println!(
            "Closed tabbie {}. Active tabbie is now {}",
            closed_tab + 1,
            self.active_tab + 1,
        );
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
        for tab in &mut self.tabs {
            if tab.process_output() {
                received_output = true;
            }
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
        let Some(window) = &self.window.clone() else {
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
                self.renderer.resize(
                    size.width,
                    size.height,
                );

                let padding =
                    self.config.window.padding;

                let cell_width =
                    self.config.font.cell_width;

                let cell_height =
                    self.config.font.cell_height;

                let usable_width =
                    size.width.saturating_sub(
                        padding.saturating_mul(2)
                    );

                let usable_height =
                    size.height.saturating_sub(TAB_BAR_HEIGHT)
                    .saturating_sub(
                        padding.saturating_mul(2)
                    );

                let columns =
                    (usable_width / cell_width).max(1);

                let rows =
                    (usable_height / cell_height).max(1);

                for tab in &mut self.tabs {
                    tab.resize(
                        columns as usize,
                        rows as usize,
                    );
                }

                println!(
                    "Aster resized: {} x {} pixels -> {} x {} cells",
                    size.width,
                    size.height,
                    columns,
                    rows,
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

                let background = parse_color(&self.theme.colors.background);
                let foreground = parse_color(&self.theme.colors.foreground);
                let cursor = parse_color(&self.theme.colors.cursor);
                
                self.renderer.clear(background);

                self.renderer.draw_tab_bar(
                    self.tabs.len(),
                    self.active_tab,
                    TAB_BAR_HEIGHT,
                    14.0,
                    foreground,
                    background,
                    cursor,
                );

                let padding = self.config.window.padding;
                let pane_x = padding;
                let pane_y = TAB_BAR_HEIGHT + padding;

                let pane_width = size.width.saturating_sub(
                    padding.saturating_mul(2)
                );

                let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                .saturating_sub(
                    padding.saturating_mul(2)
                );

                let active_tab = self.active_tab;

                let tab = &self.tabs[active_tab];

                let focused_pane_id = tab.focused_pane_id();

                tab.for_each_visible_pane(
                    pane_x,
                    pane_y,
                    pane_width,
                    pane_height,
                    &mut | 
                        pane,
                        x,
                        y,
                        width,
                        height,
                    | {
                        if pane.id() == 
                            focused_pane_id
                        {
                            self.renderer.draw_pane_border(
                                x,
                                y,
                                width,
                                height,
                                cursor,
                            );
                        }

                        self.renderer.draw_terminal(
                            pane.terminal(),
                            x+2,
                            y+2,
                            self.config.font.cell_width,
                            self.config.font.cell_height,
                            self.config.font.size,
                            foreground,
                            background,
                            cursor,
                            &self.theme.palette,
                        );
                    }
                );

                if tab.zoomed_pane().is_none() {
                    tab.for_each_separator(
                        pane_x,
                        pane_y,
                        pane_width,
                        pane_height,
                        &mut |separator| {
                            self.renderer.draw_pane_separator(
                                separator.x,
                                separator.y,
                                separator.width,
                                separator.height,
                                foreground,
                            );
                        },
                    );
                }

                let mut buffer = surface
                    .buffer_mut()
                    .expect("Failed to access window buffer");

                buffer.copy_from_slice(self.renderer.pixels());

                window.pre_present_notify();

                buffer  
                    .present()
                    .expect("Failed to present frame")
            }

            WindowEvent::CursorMoved {
                position,
                ..
            } => {
                self.mouse_x = position.x;
                self.mouse_y = position.y;

                if self.selecting_text {
                    let size = window.inner_size();
                    let padding = self.config.window.padding;
                    let pane_x = padding;
                    let pane_y = TAB_BAR_HEIGHT + padding;

                    let pane_width = size.width.saturating_sub(
                        padding.saturating_mul(2)
                    );

                    let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                    .saturating_sub(padding.saturating_mul(2));

                    if let Some(rect) = self.active_tab()
                    .visible_pane_at_position(
                        position.x,
                        position.y,
                        pane_x,
                        pane_y,
                        pane_width,
                        pane_height,
                    )

                    {
                        let local_x = position.x - rect.x as f64 - 2.0;
                        let local_y = position.y - rect.y as f64 - 2.0;

                        if local_x >= 0.0 && local_y >= 0.0 {
                            let cell_x = (local_x / self.config.font.cell_width as f64) as usize;
                            let cell_y = (local_y / self.config.font.cell_height as f64) as usize;

                            self.active_tab_mut().terminal_mut().update_selection(
                                cell_x,
                                cell_y,
                            );

                            window.request_redraw();
                        }
                    }
                    return;
                }

                let Some(drag) = self.dragging_split 
                else {
                    return;
                };

                let size = window.inner_size();
                let padding = self.config.window.padding;

                let pane_x = padding;
                let pane_y = TAB_BAR_HEIGHT + padding;

                let pane_width = size.width.saturating_sub(
                    padding.saturating_mul(2)
                );

                let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                .saturating_sub(padding.saturating_mul(2));

                let mouse_x = self.mouse_x;
                let mouse_y = self.mouse_y;

                let changed = self.active_tab_mut().resize_split_at_position(
                    drag.split_id,
                    mouse_x,
                    mouse_y,
                    pane_x,
                    pane_y,
                    pane_width,
                    pane_height,
                );

                if changed {
                    let cell_width = self.config.font.cell_width;
                    let cell_height = self.config.font.cell_height;

                    let columns = (pane_width / cell_width).max(1) as usize;
                    let rows = (pane_height / cell_height).max(1) as usize;

                    self.active_tab_mut().resize(
                        columns,
                        rows,
                    );

                    window.request_redraw();
                }
            }

            WindowEvent::MouseInput {
                state,
                button,
                ..
            } => {
                if button != MouseButton::Left {
                    return;
                }

                if state == ElementState::Released {
                    self.dragging_split = None;
                    self.selecting_text = false;
                    return;
                }

                let size = window.inner_size();
                let padding = self.config.window.padding;

                let pane_x = padding;
                let pane_y = TAB_BAR_HEIGHT + padding;

                let pane_width = size.width.saturating_sub(
                    padding.saturating_mul(2)
                );

                let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                .saturating_sub(
                    padding.saturating_mul(2)
                );

                let mouse_x = self.mouse_x;
                let mouse_y = self.mouse_y;

                if let Some(separator) = self.active_tab().separator_at_position(
                    mouse_x,
                    mouse_y,
                    pane_x,
                    pane_y,
                    pane_width,
                    pane_height,
                )

                {
                    self.dragging_split = Some(
                        SplitDrag {
                            split_id: separator.split_id,
                        }
                    );

                    println!(
                        "Started dragging split {}",
                        separator.split_id,
                    );

                    return;
                }

                if let Some(rect) = self.active_tab().visible_pane_at_position(
                    mouse_x,
                    mouse_y,
                    pane_x,
                    pane_y,
                    pane_width,
                    pane_height,
                )

                {
                    let cell_width = self.config.font.cell_width;
                    let cell_height = self.config.font.cell_height;

                    let local_x = mouse_x - rect.x as f64 - 2.0;
                    let local_y = mouse_y - rect.y as f64 - 2.0;

                    if local_x >= 0.0 && local_y >= 0.0 {
                        let cell_x = (local_x / cell_width as f64) as usize;
                        let cell_y = (local_y / cell_height as f64) as usize;

                        self.active_tab_mut().focus_at_position(
                            mouse_x,
                            mouse_y,
                            pane_x,
                            pane_y,
                            pane_width,
                            pane_height,
                        );

                        self.active_tab_mut()
                            .terminal_mut()
                            .start_selection(
                                cell_x,
                                cell_y,
                            );

                        self.selecting_text = true;
                        window.request_redraw();
                        return;
                    }
                }

                let changed =
                    self.active_tab_mut().focus_at_position(
                        mouse_x,
                        mouse_y,
                        pane_x,
                        pane_y,
                        pane_width,
                        pane_height,
                    );

                if changed {
                    window.request_redraw();
                }
            }

            WindowEvent::MouseWheel { delta, ..} => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        y.round() as i32
                    }

                    MouseScrollDelta::PixelDelta(position) => {
                        (
                            position.y / self.config.font.cell_height as f64
                        ).round() as i32
                    }
                };

                if lines > 0 {
                    self.active_tab_mut().terminal_mut()
                    .scroll_view_up(lines as usize);
                } else if lines < 0 {
                    self.active_tab_mut().terminal_mut()
                    .scroll_view_down((-lines) as usize);
                }

                window.request_redraw();
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                let control = self.modifiers.control_key();
                let shift = self.modifiers.shift_key();

                if control && shift {
                    if let Key::Character(character) = &event.logical_key {
                        if character.eq_ignore_ascii_case("t") {
                            self.new_tab();
                            window.request_redraw();
                            return;
                        }

                        if character.eq_ignore_ascii_case("w") {
                            self.close_active_tab();
                            window.request_redraw();
                            return;
                        }

                        if character.eq_ignore_ascii_case("d") {
                            self.active_tab_mut()
                                .split_active(
                                    SplitDirection::Vertical
                                );

                            window.request_redraw();
                            return;
                        }

                        if character.eq_ignore_ascii_case("e") {
                            self.active_tab_mut()
                                .split_active(
                                    SplitDirection::Horizontal
                                );

                            window.request_redraw();
                            return;
                        }

                        if character.eq_ignore_ascii_case("q") {
                            self.active_tab_mut().close_active_pane();
                            window.request_redraw();
                            return;
                        }

                        if character.eq_ignore_ascii_case("z") {
                            self.active_tab_mut().toggle_zoom();
                            let size = window.inner_size();
                            let padding = self.config.window.padding;

                            let pane_width = size.width.saturating_sub(
                                padding.saturating_mul(2)
                            );

                            let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                            .saturating_sub(padding.saturating_mul(2));

                            let columns = (pane_width / self.config.font.cell_width).max(1) as usize;
                            let rows = (pane_height / self.config.font.cell_height).max(1) as usize;

                            if let Some(pane_id) = self.active_tab().zoomed_pane() {
                                if let Some(pane) =
                                    self.active_tab_mut().root_mut().find_pane_mut(pane_id)
                                {
                                    pane.resize(
                                        columns,
                                        rows,
                                    );
                                }
                            } else {
                                self.active_tab_mut().resize(columns, rows,);
                            }

                            window.request_redraw();
                            return;
                        }

                        if character.eq_ignore_ascii_case("c") {
                            if let Some(text) = self.active_tab().terminal().selected_text()
                            {
                                if let Ok(mut clipboard) = Clipboard::new() {
                                    let _ = clipboard.set_text(text);
                                }
                            }
                            return;
                        }

                        if character.eq_ignore_ascii_case("v") {
                            if let Ok(mut clipboard) = Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    self.active_tab_mut().write(&text);
                                }
                            }
                            return;
                        }
                    }

                }

                if control && shift {
                    let padding = self.config.window.padding;

                    let size = window.inner_size();
                    let pane_x = padding;
                    let pane_y = TAB_BAR_HEIGHT + padding;
                    let pane_width =
                        size.width.saturating_sub(
                            padding.saturating_mul(2)
                        );
                    let pane_height = size.height 
                        .saturating_sub(TAB_BAR_HEIGHT)
                        .saturating_sub(padding.saturating_mul(2));

                    let direction = 
                        match &event.logical_key {
                            Key::Named(
                                NamedKey::ArrowLeft
                            ) => Some(FocusDirection::Left),

                            Key::Named(
                                NamedKey::ArrowRight
                            ) => Some(FocusDirection::Right),

                            Key::Named(
                                NamedKey::ArrowUp
                            ) => Some(FocusDirection::Up),

                            Key::Named(
                                NamedKey::ArrowDown
                            ) => Some(FocusDirection::Down),

                            _ => None,
                        };

                    if let Some(direction) = direction {
                        self.active_tab_mut().focus_direction(
                            direction,
                            pane_x,
                            pane_y,
                            pane_width,
                            pane_height,
                        );

                        window.request_redraw();
                        return;
                    }
                }

                if control && shift && matches!(
                    event.logical_key,
                    Key::Named(NamedKey::Tab)
                ) {
                    self.previous_tab();
                    window.request_redraw();
                    return;
                }

                if control && !shift && matches!(
                    event.logical_key,
                    Key::Named(NamedKey::Tab)
                ) {
                    self.next_tab();
                    window.request_redraw();
                    return;
                }
                
                match &event.logical_key {
                    Key::Named(NamedKey::Enter) => {
                        self.active_tab_mut().write("\r");
                    }

                    Key::Named(NamedKey::Backspace) => {
                        self.active_tab_mut().write("\u{8}");
                    }

                    Key::Named(NamedKey::ArrowUp) => {
                        self.active_tab_mut().write("\x1b[A");
                    }

                    Key::Named(NamedKey::ArrowDown) => {
                        self.active_tab_mut().write("\x1b[B");
                    }

                    Key::Named(NamedKey::ArrowRight) => {
                        self.active_tab_mut().write("\x1b[C");
                    }

                    Key::Named(NamedKey::ArrowLeft) => {
                        self.active_tab_mut().write("\x1b[D");
                    }

                    Key::Named(NamedKey::Home) => {
                        self.active_tab_mut().write("\x1b[H");
                    }

                    Key::Named(NamedKey::End) => {
                        self.active_tab_mut().write("\x1b[F");
                    }

                    Key::Named(NamedKey::Delete) => {
                        self.active_tab_mut().write("\x1b[3~");
                    }

                    _ => {
                        if let Some(text) = &event.text {
                            self.active_tab_mut().write(text);
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
    let config = Config::load("aster.toml");
    config.validate();

    let theme_path = format!("themes/{}.toml", config.theme);
    let theme = Theme::load(&theme_path);

    let event_loop = EventLoop::new().expect("Failed to create the event loop");
    let mut app = AsterApp::new(config, theme);
    event_loop.run_app(&mut app).expect("SOOO... it crashed");
}