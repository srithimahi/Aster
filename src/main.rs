mod config;
mod parser;
mod pty;
mod renderer;
mod terminal;
mod theme;
mod mux;
mod commands;

use arboard::Clipboard;
use config::{Config, parse_color};
use theme::Theme;
use mux::tab::Tab;
use mux::pane::{
    PaneInput,
    PaneKind,
    ProcessSort,
    SystemMonitorPane,
    SystemMonitorView,
};
use mux::layout::{
    FocusDirection,
    SplitDirection,
};

use std::{
    num::NonZeroU32,
    sync::Arc,
};
use commands::AsterCommand;

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
const TAB_WIDTH: u32 = 140;

const TAB_CLOSE_WIDTH: u32 = 28;
const NEW_TAB_BUTTON_WIDTH: u32 = 42;

#[derive(Clone, Copy, Debug)]
struct SplitDrag {
    split_id: usize,
}

#[derive(Clone, Debug)]
enum AsterOverlay {
    Shortcuts,
    CommandPalette {
        query: String,
        selected: usize,
    },
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
    overlay: Option<AsterOverlay>,
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
            overlay: None,
        }
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    fn new_tab(&mut self) {
        let(columns, rows,) = match self.active_tab().terminal() {
            Some(terminal) => {
                (
                    terminal.width(),
                    terminal.height(),
                )
            }

            None => {
                (80, 24)
            }
        };

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
        let index = self.active_tab;
        self.close_tab(index);
    }

    fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 {
            return;
        }

        if index >= self.tabs.len() {
            return;
        }

        self.tabs.remove(index);
        if self.active_tab > index {
            self.active_tab -= 1;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    fn execute_command(
        &mut self,
        command: AsterCommand,
        window: &Window,
    ) {
        match command {
            AsterCommand::NewTab => {
                self.new_tab();
                window.request_redraw();
            }

            AsterCommand::CloseTab => {
                self.close_active_tab();
                window.request_redraw();
            }

            AsterCommand::NextTab => {
                self.next_tab();
                window.request_redraw();
            }

            AsterCommand::PreviousTab => {
                self.previous_tab();
                window.request_redraw();
            }

            AsterCommand::SplitVertical => {
                self.active_tab_mut().split_active(
                    SplitDirection::Vertical
                );

                window.request_redraw();
            }

            AsterCommand::SplitHorizontal => {
                self.active_tab_mut().split_active(
                    SplitDirection::Horizontal
                );

                window.request_redraw();
            }

            AsterCommand::SystemMonitor => {
                self.active_tab_mut().split_system_monitor();
                let size = window.inner_size();
                let padding = self.config.window.padding;

                let pane_width = size.width.saturating_sub(
                    padding.saturating_mul(2)
                );

                let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                .saturating_sub(padding.saturating_mul(2));

                let columns = (
                    pane_width / self.config.font.cell_width
                ).max(1) as usize;

                let rows = (
                    pane_height / self.config.font.cell_height
                ).max(1) as usize;

                self.active_tab_mut().resize(
                    columns,
                    rows,
                );

                window.request_redraw();
            }

            AsterCommand::ClosePane => {
                self.active_tab_mut().close_active_pane();
                window.request_redraw();
            }

            AsterCommand::ZoomPane => {
                self.active_tab_mut().toggle_zoom();

                let size = window.inner_size();
                let padding = self.config.window.padding;
                let pane_width = size.width.saturating_sub(
                    padding.saturating_mul(2)
                );

                let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                .saturating_sub(padding.saturating_mul(2));

                let columns = (
                    pane_width / self.config.font.cell_width
                ).max(1) as usize;

                let rows = (
                    pane_height / self.config.font.cell_height
                ).max(1) as usize;

                if let Some(pane_id) = self.active_tab().zoomed_pane() {
                    if let Some(pane) = self.active_tab_mut()
                        .root_mut()
                        .find_pane_mut(
                            pane_id
                        ) {
                        pane.resize(
                            columns,
                            rows,
                        );
                    }
                } else {
                    self.active_tab_mut().resize(
                        columns,
                        rows,
                    );
                }

                window.request_redraw();
            }

            AsterCommand::FocusLeft | AsterCommand::FocusRight | AsterCommand::FocusUp | AsterCommand::FocusDown => {
                let size = window.inner_size();
                let padding = self.config.window.padding;

                let pane_x = padding;
                let pane_y = TAB_BAR_HEIGHT + padding;

                let pane_width = size.width.saturating_sub(
                    padding.saturating_mul(2)
                );  
                let pane_height = size.height.saturating_sub(TAB_BAR_HEIGHT)
                .saturating_sub(padding.saturating_mul(2));

                let direction = match command {
                    AsterCommand::FocusLeft => {
                        FocusDirection::Left
                    }

                    AsterCommand::FocusRight => {
                        FocusDirection::Right
                    }

                    AsterCommand::FocusUp => {
                        FocusDirection::Up
                    }

                    AsterCommand::FocusDown => {
                        FocusDirection::Down
                    }

                    _ => unreachable!(),
                };

                self.active_tab_mut().focus_direction(
                    direction,
                    pane_x,
                    pane_y,
                    pane_width,
                    pane_height,
                );

                window.request_redraw();
            }

            AsterCommand::Search => {
                if self.active_tab().active_pane_is_terminal() {
                    self.active_tab_mut().start_search();
                    window.request_redraw();
                }
            }

            AsterCommand::Copy => {
                if let Some(terminal) = self.active_tab().terminal() {
                    if let Some(text) = terminal.selected_text() {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            let _ = clipboard.set_text(text);
                        }
                    }
                }
            }

            AsterCommand::Paste => {
                if let Ok(mut clipboard) = Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        self.active_tab_mut().write(&text);
                    }
                }
            }

            AsterCommand::Shortcuts => {
                self.overlay = Some(AsterOverlay::Shortcuts);
                window.request_redraw();
            }

            AsterCommand::CommandPalette => {
                self.overlay = Some(
                    AsterOverlay::CommandPalette {
                        query: String::new(),
                        selected: 0,
                    }
                );
                window.request_redraw();
            }

            AsterCommand::ReloadConfig => {
                println!(
                    "Config reload coming next"
                );
            }
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

                let tab_titles: Vec<String> =
                    self.tabs
                        .iter()
                        .map(
                            |tab| {
                                tab.title()
                                    .to_string()
                            }
                        )
                        .collect();

                self.renderer.draw_tab_bar(
                    &tab_titles,
                    self.active_tab,
                    TAB_WIDTH,
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

                let search_matches = tab.search_matches();

                let current_search_match = tab.current_search_match();

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

                        match pane.kind() {
                            PaneKind::Terminal(
                                terminal_pane
                            ) => {
                                self.renderer.draw_terminal(
                                    terminal_pane.terminal(),
                                    x + 2,
                                    y + 2,
                                    self.config.font.cell_width,
                                    self.config.font.cell_height,
                                    self.config.font.size,
                                    foreground,
                                    background,
                                    cursor,
                                    &self.theme.palette,

                                    if pane.id() == focused_pane_id {
                                        search_matches
                                    } else {
                                        &[]
                                    },

                                    if pane.id() == focused_pane_id {
                                        current_search_match
                                    } else {
                                        None
                                    },
                                );
                            }

                            PaneKind::SystemMonitor(monitor) => {
                                let content_x = x + 18;
                                let mut content_y = y + 18;

                                let line_height = 26;

                                match monitor.view() {
                                    SystemMonitorView::ProcessInspector => {
                                        self.renderer.draw_text(
                                            "PROCESS INSPECTOR",
                                            content_x,
                                            content_y,
                                            self.config.font.size,
                                            cursor,
                                        );

                                        content_y += 46;

                                        if let Some(process) = monitor.selected_process() {
                                            self.renderer.draw_text(
                                                "NAME",
                                                content_x,
                                                content_y,
                                                13.0,
                                                foreground,
                                            );

                                            content_y += 22;

                                            self.renderer.draw_text(
                                                process.name(),
                                                content_x,
                                                content_y,
                                                18.0,
                                                cursor,
                                            );

                                            content_y += 42;

                                            self.renderer.draw_text(
                                                "PID",
                                                content_x,
                                                content_y,
                                                13.0,
                                                foreground,
                                            );

                                            content_y += 22;

                                            let pid_text = process.pid().to_string();

                                            self.renderer.draw_text(
                                                &pid_text,
                                                content_x,
                                                content_y,
                                                18.0,
                                                foreground,
                                            );

                                            content_y += 42;

                                            self.renderer.draw_text(
                                                "CPU USAGE",
                                                content_x,
                                                content_y,
                                                13.0,
                                                foreground,
                                            );

                                            content_y += 22;

                                            let cpu_text = format!(
                                                "{:.1}%",
                                                process.cpu_usage()
                                            );

                                            self.renderer.draw_text(
                                                &cpu_text,
                                                content_x,
                                                content_y,
                                                18.0,
                                                foreground,
                                            );

                                            let cpu_bar = SystemMonitorPane::usage_bar(
                                                process.cpu_usage(),
                                                16,
                                            );

                                            self.renderer.draw_text(
                                                &cpu_bar,
                                                content_x,
                                                content_y + 24,
                                                13.0,
                                                cursor,
                                            );

                                            content_y += 62;

                                            self.renderer.draw_text(
                                                "MEMORY",
                                                content_x,
                                                content_y,
                                                13.0,
                                                foreground,
                                            );

                                            content_y += 22;

                                            let memory_text = format!(
                                                "{:.1} MB",
                                                process.memory_mb()
                                            );

                                            self.renderer.draw_text(
                                                &memory_text,
                                                content_x,
                                                content_y,
                                                18.0,
                                                foreground,
                                            );

                                            content_y += 54;

                                            self.renderer.draw_text(
                                                "Esc / Q Back",
                                                content_x,
                                                content_y,
                                                13.0,
                                                cursor,
                                            );
                                        } else {
                                            self.renderer.draw_text(
                                                "Process no longer available",
                                                content_x,
                                                content_y,
                                                14.0,
                                                foreground,
                                            );
                                        }
                                    }

                                    SystemMonitorView::ConfirmKill => {
                                        self.renderer.draw_text(
                                            "KILL PROCESS?",
                                            content_x,
                                            content_y,
                                            self.config.font.size,
                                            cursor,
                                        );

                                        content_y += 52;

                                        if let Some(process) = monitor.pending_kill_process() {
                                            self.renderer.draw_text(
                                                "PROCESS",
                                                content_x,
                                                content_y,
                                                18.0,
                                                cursor,
                                            );

                                            content_y += 46;

                                            self.renderer.draw_text(
                                                "PID",
                                                content_x,
                                                content_y,
                                                13.0,
                                                foreground,
                                            );

                                            content_y += 22;

                                            let pid_text = process.pid().to_string();
                                            self.renderer.draw_text(
                                                &pid_text,
                                                content_x,
                                                content_y,
                                                18.0,
                                                foreground,
                                            );

                                            content_y += 58;

                                            self.renderer.draw_text(
                                                "This will terminate the process",
                                                content_x,
                                                content_y,
                                                14.0,
                                                foreground,
                                            );

                                            content_y += 42;

                                            self.renderer.draw_text(
                                                "[Y] Kill",
                                                content_x,
                                                content_y,
                                                16.0,
                                                cursor,
                                            );

                                            content_y += 28;

                                            self.renderer.draw_text(
                                                "[N / Esc] Cancel",
                                                content_x,
                                                content_y,
                                                14.0,
                                                foreground,
                                            );
                                        } else {
                                            self.renderer.draw_text(
                                                "Process no longer available",
                                                content_x,
                                                content_y,
                                                14.0,
                                                foreground,
                                            );
                                        }
                                    }

                                    SystemMonitorView::Overview => {
                                        const LABEL_TO_VALUE: u32 = 24;
                                        const VALUE_TO_BAR: u32 = 24;
                                        const SECTION_GAP: u32 = 34;

                                        self.renderer.draw_text(
                                            monitor.title(),
                                            content_x,
                                            content_y,
                                            self.config.font.size,
                                            cursor,
                                        );

                                        content_y += 42;

                                        self.renderer.draw_text(
                                            "CPU",
                                            content_x,
                                            content_y,
                                            14.0,
                                            foreground,
                                        );

                                        content_y += LABEL_TO_VALUE;

                                        let cpu_text = format!(
                                            "{:.1}%",
                                            monitor.cpu_usage()
                                        );

                                        self.renderer.draw_text(
                                            &cpu_text,
                                            content_x,
                                            content_y,
                                            18.0,
                                            foreground,
                                        );

                                        content_y += VALUE_TO_BAR;

                                        let cpu_bar = SystemMonitorPane::usage_bar(
                                            monitor.cpu_usage(),
                                            16,
                                        );

                                        self.renderer.draw_text(
                                            &cpu_bar,
                                            content_x,
                                            content_y + 24,
                                            13.0,
                                            cursor,
                                        );

                                        content_y += 24 + SECTION_GAP;

                                        self.renderer.draw_text(
                                            "MEMORY",
                                            content_x,
                                            content_y,
                                            14.0,
                                            foreground,
                                        );

                                        content_y += LABEL_TO_VALUE;

                                        let memory_text = format!(
                                            "{:.1} / {:.1} GiB",
                                            monitor.used_memory_gb(),
                                            monitor.total_memory_gb(),
                                        );

                                        self.renderer.draw_text(
                                            &memory_text,
                                            content_x,
                                            content_y,
                                            18.0,
                                            foreground,
                                        );

                                        content_y += 22;

                                        let memory_percent = format!(
                                            "{:.1}% used",
                                            monitor.memory_percentage()
                                        );

                                        self.renderer.draw_text(
                                            &memory_percent,
                                            content_x,
                                            content_y,
                                            13.0,
                                            foreground,
                                        );

                                        content_y += VALUE_TO_BAR;

                                        let memory_bar = SystemMonitorPane::usage_bar(
                                            monitor.memory_percentage(),
                                            16,
                                        );

                                        self.renderer.draw_text(
                                            &memory_bar,
                                            content_x,
                                            content_y + 24,
                                            13.0,
                                            cursor,
                                        );

                                        content_y += 24 + SECTION_GAP;

                                        self.renderer.draw_text(
                                            "DISK",
                                            content_x,
                                            content_y,
                                            14.0,
                                            foreground,
                                        );

                                        content_y += LABEL_TO_VALUE;

                                        let disk_text = format!(
                                            "{:.1} / {:.1} GiB ({:.1}%)",
                                            monitor.disk_used_gb(),
                                            monitor.disk_total_gb(),
                                            monitor.disk_usage_percent(),
                                        );

                                        self.renderer.draw_text(
                                            &disk_text,
                                            content_x,
                                            content_y,
                                            18.0,
                                            foreground,
                                        );

                                        content_y += SECTION_GAP;

                                        self.renderer.draw_text(
                                            "NETWORK",
                                            content_x,
                                            content_y,
                                            14.0,
                                            foreground,
                                        );

                                        content_y += LABEL_TO_VALUE;

                                        let download_text = format!(
                                            "↓  {}",
                                            monitor.network_received_text(),
                                        );

                                        self.renderer.draw_text(
                                            &download_text,
                                            content_x,
                                            content_y,
                                            15.0,
                                            foreground,
                                        );

                                        content_y += 22;

                                        let upload_text = format!(
                                            "↑  {}",
                                            monitor.network_transmitted_text(),
                                        );

                                        self.renderer.draw_text(
                                            &upload_text,
                                            content_x,
                                            content_y,
                                            15.0,
                                            foreground,
                                        );

                                        content_y += SECTION_GAP;

                                        self.renderer.draw_text(
                                            "UPTIME",
                                            content_x,
                                            content_y,
                                            14.0,
                                            foreground,
                                        );

                                        content_y += line_height;

                                        let uptime_text = monitor.formatted_uptime();

                                        self.renderer.draw_text(
                                            &uptime_text,
                                            content_x,
                                            content_y,
                                            18.0,
                                            foreground,
                                        );

                                        content_y += 50;

                                        let sort_label =
                                            match monitor.process_sort() {
                                                ProcessSort::Cpu => "CPU ↓",
                                                ProcessSort::Memory => "MEM ↓",
                                                ProcessSort::Name => "NAME ↑",
                                            };

                                        let process_heading = format!(
                                            "PROCESSES ({}) {}",
                                            monitor.process_count(),
                                            sort_label,
                                        );

                                        self.renderer.draw_text(
                                            &process_heading,
                                            content_x,
                                            content_y,
                                            14.0,
                                            foreground,
                                        );

                                        content_y += 30;

                                        for (index, process) in monitor
                                            .visible_processes()
                                            .iter()
                                            .enumerate()
                                        {
                                            let actual_index = monitor.process_scroll() + index;
                                            let selected = actual_index == monitor.selected_process_index();
                                            let marker = 
                                                if actual_index == monitor.selected_process_index() {
                                                    ">"
                                                } else {
                                                    " "
                                                };

                                            let process_color = if selected {
                                                cursor
                                            } else {
                                                foreground
                                            };

                                            let process_name = process
                                                    .name()
                                                    .chars()
                                                    .take(14)
                                                    .collect::<String>();

                                            let process_text = format!(
                                                "{:<14} {:>5.1}% {:>5.0}M",
                                                process_name,
                                                process.cpu_usage(),
                                                process.memory_mb(),
                                            );

                                            let row_text = format!(
                                                "{} {}",
                                                marker,
                                                process_text,
                                            );

                                            self.renderer.draw_text(
                                                &row_text,
                                                content_x,
                                                content_y,
                                                11.0,
                                                process_color,
                                            );

                                            content_y += 20;
                                        }

                                        content_y += 12;

                                        self.renderer.draw_text(
                                            "↑↓ Navigate   I Inspect   K Kill",
                                            content_x,
                                            content_y,
                                            11.0,
                                            foreground,
                                        );

                                        content_y += 20;

                                        self.renderer.draw_text(
                                            "C CPU   M Memory   N Name",
                                            content_x,
                                            content_y,
                                            11.0,
                                            cursor,
                                        )
                                    }
                                }
                            }
                        }
                    }
                );

                if let Some(overlay) = &self.overlay {
                    match overlay {
                        AsterOverlay::Shortcuts => {
                            let modal_width = 780_u32.min(size.width.saturating_sub(50));

                            let modal_height = 440_u32.min(size.height.saturating_sub(70));

                            let modal_x = size.width.saturating_sub(modal_width) / 2;

                            let modal_y = size.height.saturating_sub(modal_height) / 2;

                            let left_x = modal_x + 30;
                            let right_x = modal_x + modal_width / 2 + 10;

                            let heading_size = 15.0;
                            let label_size = 13.0;
                            let shortcut_size = 11.0;

                            self.renderer.draw_rect(
                                modal_x,
                                modal_y,
                                modal_width,
                                modal_height,
                                background,
                            );

                            self.renderer.draw_pane_border(
                                modal_x,
                                modal_y,
                                modal_width,
                                modal_height,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "A s t e r   S h o r t c u t s",
                                modal_x + 28,
                                modal_y + 20,
                                16.0,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "ESC",
                                modal_x + modal_width - 55,
                                modal_y + 20,
                                11.0,
                                foreground,
                            );

                            self.renderer.draw_rect(
                                modal_x + 26,
                                modal_y + 52,
                                modal_width.saturating_sub(52),
                                1,
                                cursor,
                            );

                            let top_y = modal_y + 75;

                            self.renderer.draw_text(
                                "T A B S",
                                left_x,
                                top_y,
                                heading_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "New Tab",
                                left_x,
                                top_y + 32,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+T",
                                left_x,
                                top_y + 50,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Close Tab",
                                left_x + 170,
                                top_y + 32,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+W",
                                left_x + 170,
                                top_y + 50,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Next / Previous",
                                left_x,
                                top_y + 82,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Tab  /  Ctrl+Shift+Tab",
                                left_x,
                                top_y + 100,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "P A N E S",
                                right_x,
                                top_y,
                                heading_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Split Vertical",
                                right_x,
                                top_y + 32,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+D",
                                right_x,
                                top_y + 50,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Split Horizontal",
                                right_x + 175,
                                top_y + 32,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+E",
                                right_x + 175,
                                top_y + 50,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Monitor / Close / Zoom",
                                right_x,
                                top_y + 82,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+M  /  Q  /  Z",
                                right_x,
                                top_y + 100,
                                shortcut_size,
                                cursor,
                            );

                            let lower_y = top_y + 145;

                            self.renderer.draw_text(
                                "N A V I G A T I O N",
                                left_x,
                                lower_y,
                                heading_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Focus Left / Right",
                                left_x,
                                lower_y + 32,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+Left / Right",
                                left_x,
                                lower_y + 50,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Focus Up / Down",
                                left_x,
                                lower_y + 78,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+Up / Down",
                                left_x,
                                lower_y + 96,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "T E R M I N A L",
                                right_x,
                                lower_y,
                                heading_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Search",
                                right_x,
                                lower_y + 32,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+F",
                                right_x,
                                lower_y + 50,
                                shortcut_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Copy / Paste",
                                right_x,
                                lower_y + 78,
                                label_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Ctrl+Shift+C / V",
                                right_x,
                                lower_y + 96,
                                shortcut_size,
                                cursor,
                            );

                            let bottom_y = modal_y + modal_height - 72;

                            self.renderer.draw_text(
                                "A S T E R",
                                left_x,
                                bottom_y,
                                heading_size,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Palette: Ctrl+Shift+P",
                                left_x,
                                bottom_y + 28,
                                shortcut_size,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Shortcuts: :shortcuts",
                                right_x,
                                bottom_y + 28,
                                shortcut_size,
                                foreground,
                            );
                        }

                        AsterOverlay::CommandPalette { .. } => {

                        }
                    }
                }

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

                if let Some(query) = tab.search_query() {
                    let search_width = 360_u32.min(pane_width);
                    let search_height = 28;
                    let search_x = pane_x + pane_width.saturating_sub(search_width);
                    let search_y = pane_y;

                    self.renderer.draw_search_bar(
                        query,
                        tab.current_search_number(),
                        tab.search_match_count(),
                        search_x,
                        search_y,
                        search_width,
                        search_height,
                        14.0,
                        foreground,
                        background,
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

                            if let Some(terminal) = self.active_tab_mut().terminal_mut() {
                                terminal.update_selection(
                                    cell_x,
                                    cell_y,
                                );
                                window.request_redraw();
                            }
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

                if self.mouse_y >= 0.0 && self.mouse_y < TAB_BAR_HEIGHT as f64 {
                    let tabs_end = self.tabs.len() as f64 * TAB_WIDTH as f64;
                    let new_tab_end = tabs_end + NEW_TAB_BUTTON_WIDTH as f64;

                    if self.mouse_x >= tabs_end && self.mouse_x < new_tab_end {
                        self.new_tab();
                        window.request_redraw();
                        return;
                    }

                    if self.mouse_x >= 0.0 && self.mouse_x < tabs_end {
                        let clicked_tab = (self.mouse_x / TAB_WIDTH as f64) as usize;
                        let position_inside_tab = self.mouse_x - clicked_tab as f64
                        * TAB_WIDTH as f64;

                        let close_button_start = TAB_WIDTH as f64 - TAB_CLOSE_WIDTH as f64;

                        if position_inside_tab >= close_button_start {
                            println!(
                                "Closing tab {}",
                                clicked_tab + 1,
                            );

                            self.close_tab(clicked_tab);
                        } else {
                            println!(
                                "Activating tab {}",
                                clicked_tab + 1,
                            );

                            self.active_tab = clicked_tab;
                        }
                        window.request_redraw();
                        return;
                    }
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

                        if let Some(terminal) = self.active_tab_mut().terminal_mut() {
                            terminal.start_selection(
                                cell_x,
                                cell_y,
                            );
                            self.selecting_text = true;
                        }

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

                if let Some(terminal) = self.active_tab_mut().terminal_mut() {
                    if lines > 0 {
                        terminal.scroll_view_up(lines as usize);
                    } else if lines < 0 {
                        terminal.scroll_view_down((-lines) as usize);
                    }
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

                if self.overlay.is_some() {
                    match &event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.overlay = None;
                            window.request_redraw();
                            return;
                        }

                        _ => {}
                    }

                    if matches!(
                            self.overlay,
                            Some(AsterOverlay::Shortcuts) 
                        ) {
                            return;
                        }
                }

                if control && shift {
                    if let Key::Character(character) = &event.logical_key {
                        let command = if character.eq_ignore_ascii_case("t") {
                            Some(AsterCommand::NewTab)
                        } else if character.eq_ignore_ascii_case("w") {
                            Some(AsterCommand::CloseTab)
                        } else if character.eq_ignore_ascii_case("d") {
                            Some(AsterCommand::SplitVertical)
                        } else if character.eq_ignore_ascii_case("e") {
                            Some(AsterCommand::SplitHorizontal)
                        } else if character.eq_ignore_ascii_case("m") {
                            Some(AsterCommand::SystemMonitor)
                        } else if character.eq_ignore_ascii_case("q") {
                            Some(AsterCommand::ClosePane)
                        } else if character.eq_ignore_ascii_case("z") {
                            Some(AsterCommand::ZoomPane)
                        } else if character.eq_ignore_ascii_case("c") {
                            Some(AsterCommand::Copy)
                        } else if character.eq_ignore_ascii_case("v") {
                            Some(AsterCommand::Paste)
                        } else if character.eq_ignore_ascii_case("f") {
                            Some(AsterCommand::Search)
                        } else if character.eq_ignore_ascii_case("p") {
                            Some(AsterCommand::Shortcuts)
                        } else {
                            None
                        };

                        if let Some(command) = command {
                            self.execute_command(
                                command,
                                window,
                            );

                            return;
                        }
                    }
                }

                if control && shift {
                    let command = match &event.logical_key {
                        Key::Named(NamedKey::ArrowLeft) => {
                            Some(AsterCommand::FocusLeft)
                        }

                        Key::Named(NamedKey::ArrowRight) => {
                            Some(AsterCommand::FocusRight)
                        }

                        Key::Named(NamedKey::ArrowUp) => {
                            Some(AsterCommand::FocusUp)
                        }

                        Key::Named(NamedKey::ArrowDown) => {
                            Some(AsterCommand::FocusDown)
                        }

                        _ => None,
                    };

                    if let Some(command) = command {
                        self.execute_command(
                            command,
                            window,
                        );
                        return;
                    }
                }

                if control && shift && matches!(
                    event.logical_key,
                    Key::Named(NamedKey::Tab)
                ) {
                    self.execute_command(
                        AsterCommand::PreviousTab,
                        window,
                    );
                    return;
                }

                if control && !shift && matches!(
                    event.logical_key,
                    Key::Named(NamedKey::Tab)
                ) {
                    self.execute_command(
                        AsterCommand::NextTab,
                        window,
                    );
                    return;
                }

                if self.active_tab().is_searching() {
                    match &event.logical_key {
                        Key::Named(
                            NamedKey::Escape
                        ) => {
                            self.active_tab_mut().close_search();
                            window.request_redraw();
                            return;
                        }

                        Key::Named(
                            NamedKey::Backspace
                        ) => {
                            self.active_tab_mut().search_backspace();
                            window.request_redraw();
                            return;
                        }

                        Key::Named(
                            NamedKey::Enter
                        ) => {
                            if self.modifiers.shift_key() {
                                self.active_tab_mut().previous_search_match();
                            } else {
                                self.active_tab_mut().next_search_match();
                            }

                            window.request_redraw();
                            return;
                        }

                        Key::Character(
                            character
                        ) => {
                            if !self.modifiers.control_key()
                             && !self.modifiers.alt_key() {
                                self.active_tab_mut().push_search_text(character);
                                window.request_redraw();
                            }

                            return;
                        }

                        _ => {
                            return;
                        }
                    }
                }

                let pane_input = match &event.logical_key {
                    Key::Named(NamedKey::ArrowUp) => {
                        Some(PaneInput::Up)
                    }

                    Key::Named(NamedKey::ArrowDown) => {
                        Some(PaneInput::Down)
                    }

                    Key::Named(NamedKey::Enter) => {
                        Some(PaneInput::Enter)
                    }

                    Key::Named(NamedKey::Escape) => {
                        Some(PaneInput::Escape)
                    }

                    Key::Character(character) => {
                        let mut chars = character.chars();

                        match(
                            chars.next(),
                            chars.next(),
                        ) {
                            (
                                Some(character),
                                None,
                            ) => {
                                Some(
                                    PaneInput::Character(character)
                                )
                            }

                            _ => None,
                        }
                    }

                    _ => None,
                };

                if let Some(input) = pane_input {
                    let handled = self.active_tab_mut()
                        .active_pane_mut()
                        .handle_input(input);
                    
                    if handled {
                        window.request_redraw();
                        return;
                    }
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