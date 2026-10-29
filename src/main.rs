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
    window::{CursorIcon, Window, WindowId},
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

    CommandLine {
        input: String,
        error: Option<String>,
    },

    RenameTab {
        input: String,
    },
}

struct AsterApp {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    tabs: Vec<Tab>,
    active_tab: usize,
    next_tab_number: usize,
    renderer: Renderer,
    config: Config,
    theme: Theme,
    current_theme: String,

    modifiers: ModifiersState,
    dragging_split: Option<SplitDrag>,
    selecting_text: bool,

    mouse_x: f64,
    mouse_y: f64,
    overlay: Option<AsterOverlay>,
    terminal_input_fresh: bool,

    command_history: Vec<String>,
    command_history_index: Option<usize>,
    command_history_draft: String,
}

impl AsterApp {
    fn new(config: Config, theme:Theme) -> Self {
        let current_theme = config.theme.clone();
        Self {
            window: None,
            surface: None,
            tabs: Vec::new(),
            active_tab: 0,
            next_tab_number: 2,
            renderer: Renderer::new(900, 600),
            config,
            theme,
            current_theme,

            modifiers: ModifiersState::empty(),
            dragging_split: None,
            selecting_text: false,

            mouse_x: 0.0,
            mouse_y: 0.0, 
            overlay: None,
            terminal_input_fresh: true,

            command_history: Vec::new(),
            command_history_index: None,
            command_history_draft: String::new(),
        }
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    fn new_tab(&mut self) {
        let (columns, rows) = if let Some(terminal) = self.active_tab().terminal() {
            (
                terminal.width(),
                terminal.height(),
            )
        } else {
            (80, 24)
        };

        let title = format!(
            "Tab {}",
            self.next_tab_number
        );

        self.next_tab_number += 1;

        self.tabs.push(
            Tab::new(
                columns,
                rows,
                title,
            )
        );

        self.active_tab = self.tabs.len() - 1;

        self.terminal_input_fresh = true;

        println!(
            "Created tab {}",
            self.active_tab + 1,
        );
    }

    fn next_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }

        self.active_tab = (self.active_tab + 1) % self.tabs.len();

        println!(
            "Switched to tabbie {}",
            self.active_tab + 1
        );
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

    fn set_theme(
        &mut self,
        name: &str,
        window: &Window,
    ) -> Result<(), String> {
        let requested_name = name
                                .trim()
                                .to_ascii_lowercase();

        if requested_name.is_empty() {
            return Err(
                "Usage: :theme <name>".to_string()
            );
        }

        let mut themes = Self::available_themes();

        themes.push("next".to_string());
        themes.push("previous".to_string());
        themes.sort();
        themes.dedup();

        let Some(actual_name) = themes
                                    .iter()
                                    .find(|theme_name| {
                                        theme_name
                                            .eq_ignore_ascii_case(
                                                &requested_name
                                            )
                                    })
        else {
            let available = if themes.is_empty() {
                "none".to_string()
            } else {
                themes.join(", ")
            };

            return Err(
                format!(
                    "Unknown theme '{}'. Available: {}",
                    requested_name,
                    available
                )
            );
        };

        let path = format!(
            "themes/{}.toml",
            actual_name
        );

        self.theme = Theme::load(&path);
        self.current_theme = actual_name.clone();

        println!(
            "Switched Aster theme to '{}'",
            self.current_theme
        );

        window.request_redraw();
        Ok(())
    }

    fn available_themes() -> Vec<String> {
        let mut themes = Vec::new();
        let Ok(entries) = std::fs::read_dir("themes") else {
            return themes;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let Some(extension) = path.extension() else {
                continue;
            };

            if extension != "toml" {
                continue;
            }

            let Some(stem) = path.file_stem() else {
                continue;
            };

            let Some(name) = stem.to_str() else {
                continue;
            };

            themes.push(name.to_string());
        }

        themes.sort_by_key(|name| {
            name.to_ascii_lowercase()
        });

        themes
    }

    fn cycle_theme(
        &mut self,
        direction: i32,
        window: &Window,
    ) -> Result<(), String> {
        let themes = Self::available_themes();

        if themes.is_empty() {
            return Err(
                "No themes were found".to_string()
            );
        }

        let current_index = themes
                                .iter()
                                .position(|theme| {
                                    theme.eq_ignore_ascii_case(
                                        &self.current_theme
                                    )
                                }).unwrap_or(0);

        let next_index = if direction >= 0 {
            (current_index + 1) % themes.len()
        } else if current_index == 0 {
            themes.len() - 1
        } else {
            current_index - 1
        };

        let next_theme = themes[next_index].clone();
        self.set_theme(
            &next_theme,
            window,
        )
    }

    fn autocomplete_command_line(
        input: &str,
    ) -> (Option<String>, Option<String>) {
        let trimmed = input.trim_start();
        if trimmed.eq_ignore_ascii_case("theme") {
            return(
                Some("theme ".to_string()),
                None,
            );
        }

        if trimmed
            .to_ascii_lowercase()
            .starts_with("theme ") {
            let theme_prefix = trimmed[6..].trim();
            let mut themes = Self::available_themes();
            themes.push("next".to_string());
            themes.push("previous".to_string());
            themes.push("prev".to_string());
            themes.sort();
            themes.dedup();

            let matches: Vec<String> = themes
                                            .into_iter()
                                            .filter(|theme| {
                                                theme  
                                                    .to_ascii_lowercase()
                                                    .starts_with(
                                                        &theme_prefix
                                                            .to_ascii_lowercase()
                                                    )
                                            }).collect();
            
            return match matches.len() {
                0 => {
                    (
                        None,
                        Some(
                            format!(
                                "No theme matches '{}'",
                                theme_prefix
                            )
                        ),
                    )
                }

                1 => {
                    (
                        Some(
                            format!(
                                "theme {}",
                                matches[0]
                            )
                        ),
                        None,
                    )
                }

                _ => {
                    (
                        None,
                        Some(
                            format!(
                                "Matches: {}",
                                matches.join(", ")
                            )
                        ),
                    )
                }
            };
        }

        let prefix = trimmed.to_ascii_lowercase();

        let mut commands: Vec<String> = AsterCommand::ALL
                                            .iter()
                                            .map(|command| {
                                                command 
                                                    .command_name()
                                                    .to_string()
                                            }).collect();

        commands.push(
            "theme".to_string()
        );

        commands.sort();
        commands.dedup();

        let matches: Vec<String> = commands
            .into_iter()
            .filter(|command| {
                command
                    .to_ascii_lowercase()
                    .starts_with(&prefix)
            }).collect();

        match matches.len() {
            0 => {
                (
                    None,
                    Some(
                        format!(
                            "No command matches '{}'",
                            trimmed
                        )
                    ),
                )
            }

            1 => {
                (
                    Some(matches[0].clone()),
                    None,
                )
            }

            _ => {
                (
                    None,
                    Some(
                        format!(
                            "Matches: {}",
                            matches.join(", ")
                        )
                    ),
                )
            }
        }
    }

    fn remember_command(
        &mut self,
        command: &str,
    ) {
        let command = command.trim();

        if command.is_empty() {
            return;
        }

        let duplicate = self.command_history
                            .last()
                            .map(|previous| {
                                previous == command
                            })
                            .unwrap_or(false);
        
        if !duplicate {
            self.command_history.push(
                command.to_string()
            );
        }

        const MAX_HISTORY: usize = 100;

        if self.command_history.len() > MAX_HISTORY {
            let overflow = self.command_history.len() - MAX_HISTORY;
            self.command_history.drain(0..overflow);
        }

        self.command_history_index = None;
        self.command_history_draft.clear();
    }

    fn older_command(
        &mut self,
        current_input: &str,
    ) -> Option<String> {
        if self.command_history.is_empty() {
            return None;
        }

        let next_index = match self.command_history_index {
            None => {
                self.command_history_draft = current_input.to_string();
                self.command_history.len() - 1
            }

            Some(index) => {
                index.saturating_sub(1)
            }
        };

        self.command_history_index = Some(next_index);
        self.command_history
            .get(next_index)
            .cloned()
    }

    fn newer_command(
        &mut self,
    ) -> Option<String> {
        let Some(index) = self.command_history_index 
        else {
            return None;
        };

        if index + 1 < self.command_history.len() {
            let next_index = index + 1;

            self.command_history_index = Some(next_index);
            return self.command_history
                    .get(next_index)
                    .cloned();
        }

        self.command_history_index = None;
        Some(
            self.command_history_draft.clone()
        )
    }

    fn reset_command_history_navigation(
        &mut self,
    ) {
        self.command_history_index = None;
        self.command_history_draft.clear();
    }

    fn execute_command(
        &mut self,
        command: AsterCommand,
        window: &Window,
    ) {
        match command {
            AsterCommand::NewTab => {
                self.new_tab();
                self.terminal_input_fresh = true;
                window.request_redraw();
            }

            AsterCommand::CloseTab => {
                self.close_active_tab();
                self.terminal_input_fresh = true;
                window.request_redraw();
            }

            AsterCommand::NextTab => {
                self.next_tab();
                self.terminal_input_fresh = true;
                window.request_redraw();
            }

            AsterCommand::PreviousTab => {
                self.previous_tab();
                self.terminal_input_fresh = true;
                window.request_redraw();
            }

            AsterCommand::SplitVertical => {
                self.active_tab_mut().split_active(
                    SplitDirection::Vertical
                );

                self.terminal_input_fresh = true;

                window.request_redraw();
            }

            AsterCommand::SplitHorizontal => {
                self.active_tab_mut().split_active(
                    SplitDirection::Horizontal
                );

                self.terminal_input_fresh = true;

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

                let size = window.inner_size();
                let padding = self.config.window.padding;

                let pane_width = size.width.saturating_sub(
                    padding.saturating_mul(2)
                );

                let pane_height = size.height
                    .saturating_sub(TAB_BAR_HEIGHT)
                    .saturating_sub(
                        padding.saturating_mul(2)
                    );

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

        let size = window.inner_size();

        let padding =
            self.config.window.padding;

        let usable_width =
            size.width.saturating_sub(
                padding.saturating_mul(2)
            );

        let usable_height =
            size.height
                .saturating_sub(TAB_BAR_HEIGHT)
                .saturating_sub(
                    padding.saturating_mul(2)
                );

        let columns = (
            usable_width
                / self.config.font.cell_width
        ).max(1) as usize;

        let rows = (
            usable_height
                / self.config.font.cell_height
        ).max(1) as usize;

        if self.tabs.is_empty() {
            self.tabs.push(
                Tab::new(
                    columns,
                    rows,
                    "Tab 1".to_string(),
                )
            );

            self.active_tab = 0;
            self.next_tab_number = 2;
        }

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
                    background,
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

                                            if let Some(message) = monitor.status_message() {
                                                content_y += 30;
                                                self.renderer.draw_text(
                                                    message,
                                                    content_x,
                                                    content_y,
                                                    13.0,
                                                    cursor,
                                                );

                                                content_y += 30;
                                            }

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

                                        if let Some(message) = monitor.status_message() {
                                            content_y += 26;

                                            self.renderer.draw_text(
                                                message,
                                                content_x,
                                                content_y,
                                                13.0,
                                                cursor,
                                            );
                                        }

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

                                        let filter_label = if monitor.process_filter().is_empty() {
                                            String::new()
                                        } else if monitor.process_filter_active() {
                                            format!(
                                                "  /{}_",
                                                monitor.process_filter()
                                            )
                                        } else {
                                            format!(
                                                "  /{}",
                                                monitor.process_filter()
                                            )
                                        };

                                        let process_heading = format!(
                                            "PROCESSES ({}/{}) {}{}",
                                            monitor.filtered_process_count(),
                                            monitor.process_count(),
                                            sort_label,
                                            filter_label,
                                        );

                                        self.renderer.draw_text(
                                            &process_heading,
                                            content_x,
                                            content_y,
                                            14.0,
                                            foreground,
                                        );

                                        content_y += 30;

                                        for process in monitor
                                            .visible_filtered_processes()
                                            .iter()
                                        {
                                            let selected = monitor.selected_pid() == Some(process.pid());
                                            
                                            let marker = if selected {
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
                                            "C CPU   M Memory   N Name   / Filter   X Clear",
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
                            let modal_width = 860_u32.min(
                                size.width.saturating_sub(50)
                            );

                            let modal_height = 610_u32.min(
                                size.height.saturating_sub(50)
                            );

                            let modal_x = size.width.saturating_sub(modal_width) / 2;

                            let modal_y = size.height.saturating_sub(modal_height) / 2;

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
                                "ASTER SHORTCUTS",
                                modal_x + 28,
                                modal_y + 20,
                                16.0,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Esc to close",
                                modal_x + modal_width - 120,
                                modal_y + 22,
                                10.0,
                                foreground,
                            );

                            self.renderer.draw_rect(
                                modal_x + 26,
                                modal_y + 55,
                                modal_width.saturating_sub(52),
                                1,
                                cursor,
                            );

                            let left_x = modal_x + 34;
                            let right_x = modal_x + modal_width / 2 + 24;

                            let heading_size = 13.0;
                            let label_size = 12.0;
                            let shortcut_size = 10.0;

                            let item_gap = 43_u32;

                            let mut y = modal_y + 78;

                            self.renderer.draw_text(
                                "TABS",
                                left_x,
                                y,
                                heading_size,
                                cursor,
                            );

                            y += 29;

                            for (label, shortcut) in [
                                ("New Tab", "Ctrl+Shift+T"),
                                ("Close Tab", "Ctrl+Shift+W"),
                                (
                                    "Next / Previous",
                                    "Ctrl+Tab / Ctrl+Shift+Tab",
                                ),
                                ("Rename Tab", "F2"),
                            ] {
                                self.renderer.draw_text(
                                    label,
                                    left_x,
                                    y,
                                    label_size,
                                    foreground,
                                );

                                self.renderer.draw_text(
                                    shortcut,
                                    left_x,
                                    y + 18,
                                    shortcut_size,
                                    cursor,
                                );

                                y += item_gap;
                            }

                            y += 8;

                            self.renderer.draw_text(
                                "NAVIGATION",
                                left_x,
                                y,
                                heading_size,
                                cursor,
                            );

                            y += 29;

                            for (label, shortcut) in [
                                (
                                    "Focus Left / Right",
                                    "Ctrl+Shift+Left / Right",
                                ),
                                (
                                    "Focus Up / Down",
                                    "Ctrl+Shift+Up / Down",
                                ),
                            ] {
                                self.renderer.draw_text(
                                    label,
                                    left_x,
                                    y,
                                    label_size,
                                    foreground,
                                );

                                self.renderer.draw_text(
                                    shortcut,
                                    left_x,
                                    y + 18,
                                    shortcut_size,
                                    cursor,
                                );

                                y += item_gap;
                            }

                            y += 8;

                            self.renderer.draw_text(
                                "TERMINAL",
                                left_x,
                                y,
                                heading_size,
                                cursor,
                            );

                            y += 29;

                            for (label, shortcut) in [
                                ("Search", "Ctrl+Shift+F"),
                                (
                                    "Copy / Paste",
                                    "Ctrl+Shift+C / Ctrl+Shift+V",
                                ),
                            ] {
                                self.renderer.draw_text(
                                    label,
                                    left_x,
                                    y,
                                    label_size,
                                    foreground,
                                );

                                self.renderer.draw_text(
                                    shortcut,
                                    left_x,
                                    y + 18,
                                    shortcut_size,
                                    cursor,
                                );

                                y += item_gap;
                            }

                            let mut y = modal_y + 78;

                            self.renderer.draw_text(
                                "PANES",
                                right_x,
                                y,
                                heading_size,
                                cursor,
                            );

                            y += 29;

                            for (label, shortcut) in [
                                (
                                    "Split Vertical",
                                    "Ctrl+Shift+D",
                                ),
                                (
                                    "Split Horizontal",
                                    "Ctrl+Shift+E",
                                ),
                                (
                                    "Close Pane",
                                    "Ctrl+Shift+Q",
                                ),
                                (
                                    "Zoom Pane",
                                    "Ctrl+Shift+Z",
                                ),
                            ] {
                                self.renderer.draw_text(
                                    label,
                                    right_x,
                                    y,
                                    label_size,
                                    foreground,
                                );

                                self.renderer.draw_text(
                                    shortcut,
                                    right_x,
                                    y + 18,
                                    shortcut_size,
                                    cursor,
                                );

                                y += item_gap;
                            }

                            // ─────────────────────────────
                            // RIGHT — TOOLS
                            // ─────────────────────────────

                            y += 8;

                            self.renderer.draw_text(
                                "TOOLS",
                                right_x,
                                y,
                                heading_size,
                                cursor,
                            );

                            y += 29;

                            for (label, shortcut) in [
                                (
                                    "System Monitor",
                                    "Ctrl+Shift+M",
                                ),
                                (
                                    "Command Palette",
                                    "Ctrl+Shift+P",
                                ),
                            ] {
                                self.renderer.draw_text(
                                    label,
                                    right_x,
                                    y,
                                    label_size,
                                    foreground,
                                );

                                self.renderer.draw_text(
                                    shortcut,
                                    right_x,
                                    y + 18,
                                    shortcut_size,
                                    cursor,
                                );

                                y += item_gap;
                            }

                            y += 8;

                            self.renderer.draw_text(
                                "ASTER COMMANDS",
                                right_x,
                                y,
                                heading_size,
                                cursor,
                            );

                            y += 29;

                            for (label, shortcut) in [
                                (
                                    "Command Mode",
                                    ":",
                                ),
                                (
                                    "Autocomplete",
                                    "Tab",
                                ),
                                (
                                    "Command History",
                                    "Up / Down",
                                ),
                            ] {
                                self.renderer.draw_text(
                                    label,
                                    right_x,
                                    y,
                                    label_size,
                                    foreground,
                                );

                                self.renderer.draw_text(
                                    shortcut,
                                    right_x,
                                    y + 18,
                                    shortcut_size,
                                    cursor,
                                );

                                y += item_gap;
                            }
                        }

                        AsterOverlay::CommandPalette {
                            query,
                            selected,
                        } => {
                            let palette_width = 820_u32.min(
                                size.width.saturating_sub(50)
                            );

                            let palette_height = 470_u32.min(
                                size.height.saturating_sub(60)
                            );

                            let palette_x = size.width.saturating_sub(palette_width) / 2;
                            let palette_y = size.height.saturating_sub(palette_height) / 2;

                            let center_x = palette_x + palette_width / 2;
                            
                            self.renderer.draw_rect(
                                palette_x,
                                palette_y,
                                palette_width,
                                palette_height,
                                background,
                            );

                            self.renderer.draw_pane_border(
                                palette_x,
                                palette_y,
                                palette_width,
                                palette_height,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Aster's Commands",
                                palette_x + 28,
                                palette_y + 20,
                                16.0,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Esc",
                                palette_x + palette_width - 55,
                                palette_y + 22,
                                10.0,
                                foreground,
                            );

                            self.renderer.draw_rect(
                                palette_x + 26,
                                palette_y + 55,
                                palette_width.saturating_sub(52),
                                1,
                                cursor,
                            );

                            let query_display = if query.is_empty() {
                                "Type to search commands.".to_string()
                            } else {
                                format!("> {}", query)
                            };

                            self.renderer.draw_text(
                                &query_display,
                                palette_x + 32,
                                palette_y + 75,
                                14.0,
                                foreground,
                            );

                            self.renderer.draw_rect(
                                palette_x + 28,
                                palette_y + 108,
                                palette_width.saturating_sub(56),
                                1,
                                cursor,
                            );

                            let matches: Vec<&AsterCommand> =
                                AsterCommand::ALL
                                    .iter()
                                    .filter(|command| {
                                        command.matches_query(query)
                                    }).collect();

                            let visible_commands = 8_usize;

                            let page_start = (*selected / visible_commands) * visible_commands;

                            let visible_matches: Vec<&AsterCommand> =
                                matches
                                    .iter()
                                    .skip(page_start)
                                    .take(visible_commands)
                                    .copied()
                                    .collect();

                            let list_top = palette_y + 132;
                            let list_bottom = palette_y + palette_height - 55;

                            self.renderer.draw_rect(
                                center_x,
                                list_top,
                                1,
                                list_bottom.saturating_sub(list_top),
                                cursor,
                            );

                            let left_x = palette_x + 32;
                            let right_x = center_x + 28;

                            let row_height = 68_u32;

                            for(visible_index, command) in visible_matches.iter().enumerate() {
                                let column = visible_index / 4;
                                let row = visible_index % 4;

                                let command_x = 
                                    if column == 0 {
                                        left_x
                                    } else {
                                        right_x
                                    };

                                let command_y = list_top + row as u32 * row_height;
                                let actual_index = page_start + visible_index;
                                let is_selected = actual_index == *selected;

                                if is_selected {
                                    let highlight_x = 
                                        if column == 0 {
                                            palette_x + 18
                                        } else {
                                            center_x + 12
                                        };

                                    let highlight_width = palette_width / 2 - 30;

                                    self.renderer.draw_rect(
                                        highlight_x,
                                        command_y.saturating_sub(5),
                                        highlight_width,
                                        52,
                                        cursor,
                                    );
                                }

                                let prefix = 
                                    if is_selected {
                                        "> "
                                    } else {
                                        "  "
                                    };

                                let label = 
                                    format!("{}{}",
                                    prefix,
                                    command.name()
                                    );

                                self.renderer.draw_text(
                                    &label,
                                    command_x,
                                    command_y,
                                    14.0,
                                    if is_selected {
                                        background
                                    } else {
                                        foreground
                                    },
                                );

                                let command_label = 
                                    format!(
                                        ":{}",
                                        command.command_name()
                                    );

                                self.renderer.draw_text(
                                    &command_label,
                                    command_x + 22,
                                    command_y + 24,
                                    12.5,
                                    if is_selected {
                                        background
                                    } else {
                                        cursor
                                    },
                                );
                            }

                            if matches.is_empty() {
                                self.renderer.draw_text(
                                    "No matching commands",
                                    palette_x + 32,
                                    list_top + 10,
                                    12.0,
                                    foreground,
                                );
                            }

                            if matches.len() > visible_commands {
                                let current_page = (*selected / visible_commands) + 1;
                                let total_pages = (matches.len() + visible_commands - 1) / visible_commands;
                                let page_text = format!(
                                    "Page {} / {}",
                                    current_page,
                                    total_pages
                                );

                                self.renderer.draw_text(
                                    &page_text,
                                    center_x - 50,
                                    palette_y + palette_height - 31,
                                    9.0,
                                    cursor,
                                );
                            }
                        }

                        AsterOverlay::CommandLine {
                            input,
                            error,
                        } => {
                            let bar_width =
                                720_u32.min(
                                    size.width.saturating_sub(60)
                                );

                            let inner_width = bar_width.saturating_sub(48);

                            let command_text = format!(":{}", input);

                            let command_font_size = 16.0;
                            let command_line_height = 23_u32;

                            let command_char_width = command_font_size as u32;

                            let command_chars_per_line =
                                (inner_width / command_char_width)
                                    .max(1) as usize;

                            let command_char_count = command_text.chars().count();

                            let command_lines =
                                (
                                    command_char_count
                                        + command_chars_per_line
                                        - 1
                                ) / command_chars_per_line;

                            let command_lines = command_lines.max(1) as u32;

                            let error_lines =
                                error.as_ref()
                                    .map(|message| {
                                        let chars_per_line =
                                            (inner_width / 11)
                                                .max(1) as usize;

                                        (
                                            message.chars().count()
                                                + chars_per_line
                                                - 1
                                        ) / chars_per_line
                                    })
                                    .unwrap_or(1)
                                    .max(1) as u32;

                            let message_height =
                                if error.is_some() {
                                    error_lines * 18
                                } else {
                                    18
                                };

                            let bar_height =
                                30
                                + command_lines * command_line_height
                                + message_height
                                + 30;

                            let bar_x =
                                size.width
                                    .saturating_sub(bar_width)
                                    / 2;

                            let bar_y =
                                size.height
                                    .saturating_sub(bar_height)
                                    .saturating_sub(35);

                            self.renderer.draw_rect(
                                bar_x,
                                bar_y,
                                bar_width,
                                bar_height,
                                background,
                            );

                            self.renderer.draw_pane_border(
                                bar_x,
                                bar_y,
                                bar_width,
                                bar_height,
                                cursor,
                            );

                            self.renderer.draw_wrapped_text(
                                &command_text,
                                bar_x + 24,
                                bar_y + 18,
                                inner_width,
                                command_font_size,
                                command_line_height,
                                foreground,
                            );

                            let cursor_character_index = command_char_count;

                            let cursor_line = cursor_character_index / command_chars_per_line;

                            let cursor_column = cursor_character_index % command_chars_per_line;

                            let cursor_x =
                                bar_x
                                + 24
                                + cursor_column as u32
                                    * command_char_width;

                            let cursor_y =
                                bar_y
                                + 17
                                + cursor_line as u32
                                    * command_line_height;

                            self.renderer.draw_rect(
                                cursor_x,
                                cursor_y,
                                2,
                                20,
                                cursor,
                            );

                            let message_y =
                                bar_y
                                + 24
                                + command_lines
                                    * command_line_height;

                            if let Some(error) = error {
                                self.renderer.draw_wrapped_text(
                                    error,
                                    bar_x + 24,
                                    message_y,
                                    inner_width,
                                    11.0,
                                    18,
                                    cursor,
                                );
                            } else {
                                self.renderer.draw_wrapped_text(
                                    "Enter Run   Tab Complete   ↑↓ History   Esc Cancel",
                                    bar_x + 24,
                                    message_y,
                                    inner_width,
                                    10.0,
                                    17,
                                    cursor,
                                );
                            }
                        }

                        AsterOverlay::RenameTab {
                            input,
                        } => {
                            let box_width = 500_u32.min(size.width.saturating_sub(60));
                            let box_height = 90;
                            let box_x = size.width.saturating_sub(box_width) / 2;
                            let box_y = size.height.saturating_sub(box_height) / 2;

                            self.renderer.draw_rect(
                                box_x,
                                box_y,
                                box_width,
                                box_height,
                                background,
                            );

                            self.renderer.draw_pane_border(
                                box_x,
                                box_y,
                                box_width,
                                box_height,
                                cursor,
                            );

                            self.renderer.draw_text(
                                "Rename Tab",
                                box_x + 22,
                                box_y + 16,
                                14.0,
                                cursor,
                            );

                            let display = format!("> {}", input);

                            self.renderer.draw_text(
                                &display,
                                box_x + 22,
                                box_y + 48,
                                14.0,
                                foreground,
                            );

                            self.renderer.draw_text(
                                "Enter Save   Esc Cancel",
                                box_x + box_width - 210,
                                box_y + 18,
                                9.0,
                                foreground,
                            );
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

                let size = window.inner_size();

                let padding = self.config.window.padding;

                let pane_x = padding;
                let pane_y = TAB_BAR_HEIGHT + padding;

                let pane_width =
                    size.width.saturating_sub(
                        padding.saturating_mul(2)
                    );

                let pane_height =
                    size.height
                        .saturating_sub(TAB_BAR_HEIGHT)
                        .saturating_sub(
                            padding.saturating_mul(2)
                        );

                let mut over_text = false;

                if let Some(rect) =
                    self.active_tab()
                        .visible_pane_at_position(
                            position.x,
                            position.y,
                            pane_x,
                            pane_y,
                            pane_width,
                            pane_height,
                        )
                {
                    let local_x =
                        position.x
                        - rect.x as f64
                        - 2.0;

                    let local_y =
                        position.y
                        - rect.y as f64
                        - 2.0;

                    if local_x >= 0.0 && local_y >= 0.0
                    {
                        let cell_x =
                            (
                                local_x
                                / self.config.font.cell_width
                                    as f64
                            ) as usize;

                        let cell_y =
                            (
                                local_y
                                / self.config.font.cell_height
                                    as f64
                            ) as usize;

                        if let Some(terminal) =
                            self.active_tab().terminal()
                        {
                            over_text =
                                terminal.can_start_selection(
                                    cell_x,
                                    cell_y,
                                );
                        }
                    }
                }

                window.set_cursor(
                    if over_text {
                        CursorIcon::Text
                    } else {
                        CursorIcon::Default
                    }
                );

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

                        if let Some(terminal) =
                            self.active_tab_mut().terminal_mut()
                        {
                            if terminal.can_start_selection(
                                cell_x,
                                cell_y,
                            ) {
                                terminal.start_selection(
                                    cell_x,
                                    cell_y,
                                );

                                self.selecting_text = true;

                                window.request_redraw();
                            }
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

                match self
                    .active_tab_mut()
                    .active_pane_mut()
                    .kind_mut()
                {
                    PaneKind::Terminal(terminal_pane) => {
                        let terminal =
                            terminal_pane.terminal_mut();

                        if lines > 0 {
                            terminal.scroll_view_up(
                                lines as usize
                            );
                        } else if lines < 0 {
                            terminal.scroll_view_down(
                                (-lines) as usize
                            );
                        }
                    }

                    PaneKind::SystemMonitor(monitor) => {
                        let amount =
                            lines.unsigned_abs()
                                .max(1)
                                .min(8);

                        for _ in 0..amount {
                            if lines > 0 {
                                monitor.select_previous_process();
                            } else if lines < 0 {
                                monitor.select_next_process();
                            }
                        }
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
                    if matches!(
                        event.logical_key,
                        Key::Named(NamedKey::Escape)
                    ) {
                        let was_command_line = matches!(
                            self.overlay,
                            Some(
                                AsterOverlay::CommandLine { .. }
                            )
                        );

                        self.overlay = None;

                        if was_command_line {
                            self.terminal_input_fresh = true;
                            self.reset_command_history_navigation();
                        }

                        window.request_redraw();
                        return;
                    }

                    if matches!(
                        self.overlay,
                        Some(AsterOverlay::Shortcuts)
                    ) {
                        return;
                    }

                    let mut command_to_execute: Option<AsterCommand> = None;
                    let mut close_command_line = false;
                    let mut command_line_to_execute: Option<String> = None;

                    let mut renamed_tab: Option<String> = None;

                    if let Some(
                        AsterOverlay::RenameTab {
                            input,
                        }
                    ) = &mut self.overlay {
                        match &event.logical_key {
                            Key::Named(NamedKey::Backspace) => {
                                input.pop();
                                window.request_redraw();
                                return;
                            }

                            Key::Named(NamedKey::Enter) => {
                                let new_title = input.trim().to_string();

                                if !new_title.is_empty() {
                                    renamed_tab = Some(new_title);
                                }
                            }

                            Key::Character(character) => {
                                if !control && !self.modifiers.alt_key() {
                                    input.push_str(character);
                                    window.request_redraw();
                                }

                                return;
                            }

                            _ => {
                                return;
                            }
                        }
                    }

                    if let Some(new_title) = renamed_tab {
                        self.active_tab_mut().set_title(new_title);

                        self.overlay = None;
                        window.request_redraw();
                        return;
                    }

                    let mut history_direction: Option<i32> = None;
                    let mut history_current_input = String::new();

                    if let Some(
                        AsterOverlay::CommandLine {
                            input,
                            error,
                        }
                    ) = &mut self.overlay {
                        match &event.logical_key {
                            Key::Named(NamedKey::Backspace) => {
                                if input.is_empty() {
                                    close_command_line = true;
                                } else {
                                    input.pop();
                                    *error = None;
                                }
                                window.request_redraw();
                            }

                            Key::Named(NamedKey::Enter) => {
                                let command_text = input.trim().to_string();

                                if command_text.is_empty() {
                                    close_command_line = true;
                                } else {
                                    command_line_to_execute = Some(command_text);
                                }

                                window.request_redraw();
                            }

                            Key::Named(NamedKey::Tab) => {
                                let (
                                    completion,
                                    completion_message,
                                ) = Self::autocomplete_command_line(
                                    input
                                );

                                if let Some(completed) = completion {
                                    *input = completed;
                                    *error = None;
                                } else {
                                    *error = completion_message;
                                }

                                window.request_redraw();
                                return;
                            }

                            Key::Named(NamedKey::ArrowUp) => {
                                history_current_input = input.clone();
                                history_direction = Some(-1);

                                *error = None;
                                window.request_redraw();
                            }

                            Key::Named(NamedKey::ArrowDown) => {
                                history_current_input = input.clone();

                                history_direction = Some(1);
                                *error = None;
                                window.request_redraw();
                            }

                            Key::Character(character) => {
                                if !control && !self.modifiers.alt_key() {
                                    input.push_str(character);
                                    *error = None;
                                    window.request_redraw();
                                }
                            }

                            _ => {
                                if !control && !self.modifiers.alt_key() {
                                    if let Some(text) = &event.text {
                                        if !text.is_empty() {
                                            input.push_str(text);
                                            *error = None;
                                            window.request_redraw();
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(direction) = history_direction {
                        let replacement = if direction < 0 {
                            self.older_command(
                                &history_current_input
                            )
                        } else {
                            self.newer_command()
                        };

                        if let Some(command) = replacement {
                            if let Some(
                                AsterOverlay::CommandLine {
                                    input,
                                    error,
                                }
                            ) = &mut self.overlay {
                                *input = command;
                                *error = None;
                            }
                        }

                        window.request_redraw();
                        return;
                    }


                    if let Some(command_text) = command_line_to_execute {
                        self.remember_command(&command_text); 
                        self.terminal_input_fresh = true;
                        let mut parts = command_text.split_whitespace();

                        let command_name = parts.next().unwrap_or("");
                        if command_name.eq_ignore_ascii_case("theme") {
                            let theme_name = parts.next();
                            let result = match theme_name {
                                Some(name)
                                    if parts.next().is_some() => {
                                        Err(
                                            "Usage: :theme [name|next|previous]".to_string()
                                        )
                                    }

                                    Some(name)
                                        if name.eq_ignore_ascii_case("next") => {
                                            self.cycle_theme(
                                                1,
                                                window,
                                            )
                                        }

                                        Some(name) 
                                            if name.eq_ignore_ascii_case("previous")
                                            || name.eq_ignore_ascii_case("prev") => {
                                                self.cycle_theme(
                                                    -1,
                                                    window,
                                                )
                                            }

                                        Some(name) => {
                                            self.set_theme(
                                                name, 
                                                window,
                                            )
                                        }

                                        None => {
                                            let themes = Self::available_themes();
                                            let available = if themes.is_empty() {
                                                "none".to_string()
                                            } else {
                                                themes.iter()
                                                    .map(|theme| {
                                                        if theme.eq_ignore_ascii_case(&self.current_theme) {
                                                            format!("[{}]", theme)
                                                        } else {
                                                            theme.clone()
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()
                                                    .join(", ")
                                            };

                                            Err(
                                                format!(
                                                    "Current: {} | Available: {}",
                                                    self.current_theme,
                                                    available
                                                )
                                            )
                                        }
                            };

                            match result {
                                Ok(()) => {
                                    self.overlay = None;
                                    self.terminal_input_fresh = true;
                                }

                                Err(message) => {
                                    self.overlay = Some(
                                        AsterOverlay::CommandLine {
                                            input: command_text,
                                            error: Some(message),
                                        }
                                    );
                                }
                            }

                            window.request_redraw();
                            return;
                        }

                        if parts.next().is_some() {
                            self.overlay = Some(
                                AsterOverlay::CommandLine {
                                    input: command_text.clone(),
                                    error: Some(
                                        format!(
                                            ":{} doesn't take arguments",
                                            command_name
                                        )
                                    ),
                                }
                            );

                            window.request_redraw();
                            return;
                        }

                        if let Some(command) = AsterCommand::from_command_name(
                            command_name
                        ) {
                            self.overlay = None;
                            self.terminal_input_fresh = true;

                            self.execute_command(
                                command,
                                window,
                            );

                            return;
                        }

                        self.overlay = Some(
                            AsterOverlay::CommandLine {
                                input: command_text.clone(),
                                error: Some(
                                    format!(
                                        "Unknown Aster command: :{}",
                                        command_name
                                    )
                                ),
                            }
                        );

                        window.request_redraw();
                        return;
                    }

                    if close_command_line {
                        self.overlay = None;
                        self.terminal_input_fresh = true;
                        window.request_redraw();
                        return;
                    }

                    if let Some(command) = command_to_execute {
                        self.overlay = None;
                        self.terminal_input_fresh = true;
                        self.execute_command(
                            command,
                            window,
                        );
                        return;
                    }

                    if matches!(
                        self.overlay,
                        Some(AsterOverlay::CommandLine { .. })
                    ) {
                        return;
                    }

                    if let Some(
                        AsterOverlay::CommandPalette {
                            query,
                            selected,
                        }
                    ) = &mut self.overlay {
                        match &event.logical_key {
                            Key::Named(NamedKey::Backspace) => {
                                query.pop();
                                *selected = 0;

                                window.request_redraw();
                                return;
                            }

                            Key::Named(NamedKey::ArrowUp) => {
                                let match_count = AsterCommand::ALL 
                                    .iter()
                                    .filter(|command| {
                                        command.matches_query(query)
                                    }).count();

                                if match_count > 0 {
                                    if *selected == 0 {
                                        *selected = match_count - 1;
                                    } else {
                                        *selected -= 1;
                                    }
                                }

                                window.request_redraw();
                                return;
                            }

                            Key::Named(NamedKey::ArrowDown) => {
                                let match_count = AsterCommand::ALL
                                    .iter()
                                    .filter(|command| {
                                        command.matches_query(query)
                                    }).count();

                                if match_count > 0 {
                                    *selected = (*selected + 1) % match_count;
                                }

                                window.request_redraw();
                                return;
                            }

                            Key::Named(NamedKey::Enter) => {
                                let command = AsterCommand::ALL 
                                    .iter()
                                    .filter(|command| {
                                        command.matches_query(query)
                                    }).nth(*selected)
                                    .copied();

                                if let Some(command) = command {
                                    self.overlay = None;
                                    self.execute_command(
                                        command,
                                        window,
                                    );
                                }

                                return;
                            }

                            Key::Character(character) => {
                                if !control && !self.modifiers.alt_key() {
                                    query.push_str(character);
                                    *selected = 0;
                                    window.request_redraw();
                                }

                                return;
                            }

                            _ => {
                                return;
                            }
                        }
                    }
                }

                if matches!(
                    event.logical_key,
                    Key::Named(NamedKey::F2)
                ) {
                    let current_title = 
                        self.active_tab()
                            .title()
                            .to_string();

                    self.overlay = Some(AsterOverlay::RenameTab {
                        input: current_title,
                    });

                    window.request_redraw();
                    return;
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
                            Some(AsterCommand::CommandPalette)
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

                if self.terminal_input_fresh && self.active_tab().active_pane_is_terminal() {
                    if let Key::Character(character) = &event.logical_key {
                        if character == ":" && !control && !self.modifiers.alt_key() {
                            self.reset_command_history_navigation();
                            self.terminal_input_fresh = true;

                            self.overlay = Some(AsterOverlay:: CommandLine {
                                input: String::new(),
                                error: None,
                            });

                            window.request_redraw();
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

                    Key::Named(NamedKey::Backspace) => {
                        Some(PaneInput::Backspace)
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
                        self.terminal_input_fresh = true;
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

                            if !text.is_empty() {
                                self.terminal_input_fresh = false;
                            }
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