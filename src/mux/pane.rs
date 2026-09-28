use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};

use std::time::{
    Duration,
    Instant,
};

use sysinfo::{
    Disks,
    Networks,
    Pid,
    System,
};

use crate::{
    parser::{
        AnsiParser,
        ParserEvent,
    },
    pty::PtySession,
    terminal::Terminal,
};

static NEXT_PANE_ID: AtomicUsize = AtomicUsize::new(1);

pub struct Pane {
    id: usize,
    kind: PaneKind,
}

impl Pane {
    pub fn new(
        columns: usize,
        rows: usize,
    ) -> Self {
        Self::new_terminal(
            columns,
            rows,
        )
    }

    pub fn new_terminal(
        columns: usize,
        rows: usize,
    ) -> Self {
        Self {
            id: NEXT_PANE_ID.fetch_add(
                1,
                Ordering::Relaxed,
            ),

            kind: PaneKind::Terminal(
                TerminalPane::new(
                    columns,
                    rows,
                )
            ),
        }
    }

    pub fn new_system_monitor() -> Self {
        Self {
            id: NEXT_PANE_ID.fetch_add(
                1,
                Ordering::Relaxed,
            ),

            kind: PaneKind::SystemMonitor(
                SystemMonitorPane::new()
            ),
        }
    }

    pub fn pane_type(
        &self,
    ) -> PaneType {
        match &self.kind {
            PaneKind::Terminal(_) => {
                PaneType::Terminal
            }

            PaneKind::SystemMonitor(_) => {
                PaneType::SystemMonitor
            }
        }
    }

    pub fn is_terminal(
        &self,
    ) -> bool {
        matches!(
            self.kind,
            PaneKind::Terminal(_)
        )
    }

    pub fn system_monitor(
        &self,
    ) -> Option<&SystemMonitorPane> {
        match &self.kind {
            PaneKind::SystemMonitor(
                monitor
            ) => {
                Some(monitor)
            }

            _ => None,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn kind(&self) -> &PaneKind {
        &self.kind
    }

    pub fn kind_mut(
        &mut self,
    ) -> &mut PaneKind {
        &mut self.kind
    }

    pub fn title(
        &self,
    ) -> &str {
        match &self.kind {
            PaneKind::Terminal(
                terminal_pane
            ) => {
                terminal_pane.title()
            }

            PaneKind::SystemMonitor(
                monitor
            ) => {
                monitor.title()
            }
        }
    }

    pub fn terminal(&self,) -> Option<&Terminal> {
        match &self.kind {
            PaneKind::Terminal(
                terminal_pane
            ) => {
                Some(
                    terminal_pane.terminal()
                )
            }

            PaneKind::SystemMonitor(_) => {
                None
            }
        }
    }

    pub fn terminal_mut(
        &mut self,
    ) -> Option<&mut Terminal> {
        match &mut self.kind {
            PaneKind::Terminal(
                terminal_pane
            ) => {
                Some(
                    terminal_pane.terminal_mut()
                )
            }

            PaneKind::SystemMonitor(_) => {
                None
            }
        }
    }

    pub fn write(
        &mut self,
        text: &str,
    ) {
        match &mut self.kind {
            PaneKind::Terminal(
                terminal_pane
            ) => {
                terminal_pane.write(
                    text
                );
            }

            PaneKind::SystemMonitor(_) => {}
        }
    }

    pub fn resize(
        &mut self,
        columns: usize,
        rows: usize,
    ) {
        match &mut self.kind {
            PaneKind::Terminal(
                terminal_pane
            ) => {
                terminal_pane.resize(
                    columns,
                    rows,
                );
            }

            PaneKind::SystemMonitor(_) => {}
        }
    }

    pub fn process_output(
        &mut self,
    ) -> bool {
        match &mut self.kind {
            PaneKind::Terminal(
                terminal_pane
            ) => {
                terminal_pane.process_output()
            }

            PaneKind::SystemMonitor(
                monitor
            ) => {
                monitor.update()
            }
        }
    }

    pub fn handle_input(
        &mut self,
        input: PaneInput,
    ) -> bool {
        match &mut self.kind {
            PaneKind::Terminal(_) => {
                false
            }

            PaneKind::SystemMonitor(
                monitor
            ) => {
                monitor.handle_input(
                    input
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemMonitorView {
    Overview,
    ProcessInspector,
    ConfirmKill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessSort {
    Cpu,
    Memory,
    Name,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneInput {
    Up,
    Down,
    Enter,
    Escape,
    Character(char),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneType {
    Terminal,
    SystemMonitor,
}

pub enum PaneKind {
    Terminal(TerminalPane),
    SystemMonitor(SystemMonitorPane),
}

pub struct TerminalPane {
    title: String,
    terminal: Terminal,
    pty: PtySession,
    parser: AnsiParser,
}

impl TerminalPane {
    pub fn new(
        columns: usize,
        rows: usize,
    ) -> Self {
        Self {
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

    pub fn title(
        &self,
    ) -> &str {
        &self.title
    }

    pub fn terminal(
        &self,
    ) -> &Terminal {
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

    fn handle_parser_event(
        &mut self,
        event: ParserEvent,
    ) {
        match event {
            ParserEvent::SetTitle(
                title
            ) => {
                self.title = title;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    name: String,
    pid: u32,
    cpu_usage: f32,
    memory: u64,
}

impl ProcessInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub fn memory(&self) -> u64 {
        self.memory
    }

    pub fn memory_mb(&self) -> f64 {
        self.memory as f64 / 1024.0 / 1024.0
    }
}

const VISIBLE_PROCESS_ROWS: usize = 8;

pub struct SystemMonitorPane {
    title: String,
    
    system: System,
    disks: Disks,
    networks: Networks,

    cpu_usage: f32,

    used_memory: u64,
    total_memory: u64,

    disk_total: u64,
    disk_available: u64,

    network_received: u64,
    network_transmitted: u64,

    uptime_seconds: u64,

    process_count: usize,
    processes: Vec<ProcessInfo>,

    selected_pid: Option<u32>,
    pending_kill_pid: Option<u32>,

    process_sort: ProcessSort,
    process_scroll: usize,

    last_refresh: Instant,
    refresh_interval: Duration,

    view: SystemMonitorView,
}

impl SystemMonitorPane {
    pub fn new() -> Self {
        let mut system = System::new_all();

        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();

        disks.refresh(true);
        networks.refresh(true);

        system.refresh_memory();

        Self {
            title: String::from(
                "System Monitor"
            ),

            cpu_usage: 0.0,
            used_memory: system.used_memory(),
            total_memory: system.total_memory(),

            disks,
            networks,

            disk_total: 0,
            disk_available: 0,

            network_received: 0,
            network_transmitted: 0,

            uptime_seconds: System::uptime(),
            process_count: 0,
            processes: Vec::new(),

            selected_pid: None,
            pending_kill_pid: None,

            process_sort: ProcessSort::Cpu,
            process_scroll: 0,

            system,

            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(1),

            view: SystemMonitorView::Overview,
        }
    }

    pub fn title(
        &self,
    ) -> &str {
        &self.title
    }

    pub fn update(
        &mut self,
    ) -> bool {
        if self.last_refresh.elapsed() < self.refresh_interval {
            return false;
        }

        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.refresh_disk_stats();
        self.refresh_network_stats();

        self.system.refresh_processes(
            sysinfo::ProcessesToUpdate::All,
            true,
        );

        self.cpu_usage = self.system.global_cpu_usage();
        self.used_memory = self.system.used_memory();
        self.total_memory = self.system.total_memory();
        self.process_count = self.system.processes().len();

        let mut processes: Vec<ProcessInfo> = self.system
            .processes()
            .iter()
            .map(
                |(pid, process)| {
                    ProcessInfo {
                        name: process
                            .name()
                            .to_string_lossy()
                            .to_string(),

                        pid: pid.as_u32(),
                        cpu_usage: process.cpu_usage(),
                        memory: process.memory(),
                    }
                },
            ).collect();

        processes.sort_by(
            |a, b| {
                b.cpu_usage.partial_cmp(
                    &a.cpu_usage
                ).unwrap_or(
                    std::cmp::Ordering::Equal
                )
            },
        );

        self.processes = processes;
        self.sort_processes();
        match self.selected_pid {
            Some(pid) => {
                let still_visible = self.processes.iter().any(
                    |process| {
                        process.pid() == pid
                    }
                );

                if !still_visible {
                    if self.view == SystemMonitorView::Overview {
                        self.selected_pid = self.processes
                            .first()
                            .map(
                                |process| {
                                    process.pid()
                                }
                            );
                    }
                }
            }

            None => {
                self.selected_pid = self.processes
                    .first()
                    .map(
                        |process| {
                            process.pid()
                        }
                    );
            }
        }

        self.keep_selection_visible();

        self.uptime_seconds = System::uptime();
        self.last_refresh = Instant::now();
        true
    }

    pub fn cpu_usage(
        &self,
    ) -> f32 {
        self.cpu_usage
    }

    pub fn used_memory(
        &self,
    ) -> u64 {
        self.used_memory
    }

    pub fn total_memory(
        &self,
    ) -> u64 {
        self.total_memory
    }

    fn refresh_disk_stats(
        &mut self,
    ) {
        self.disks.refresh(true);

        let mut total = 0_u64;
        let mut available = 0_u64;

        for disk in self.disks.list() {
            total = 
                total.saturating_add(
                    disk.total_space()
                );
            available = 
                available.saturating_add(
                    disk.available_space()
                );
        }

        self.disk_total = total;
        self.disk_available = available;
    }

    pub fn disk_total_gb(
        &self,
    ) -> f64 {
        self.disk_total as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn disk_available_gb(
        &self,
    ) -> f64 {
        self.disk_available as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn disk_used_gb(
        &self,
    ) -> f64 {
        self.disk_total.saturating_sub(
            self.disk_available
        ) as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn disk_usage_percent(
        &self,
    ) -> f32 {
        if self.disk_total == 0 {
            return 0.0;
        }

        let used = self.disk_total.saturating_sub(
            self.disk_available
        );

        (
            used as f64 / self.disk_total as f64 * 100.0
        ) as f32
    }

    fn refresh_network_stats(
        &mut self,
    ) {
        self.networks.refresh(true);
        let mut received = 0_u64;
        let mut transmitted = 0_u64;

        for(_name, data) in &self.networks {
            received = received.saturating_add(
                data.received()
            );

            transmitted = transmitted.saturating_add(
                data.transmitted()
            );
        }

        self.network_received = received;
        self.network_transmitted = transmitted;
    }

    pub fn network_received_bytes(
        &self,
    ) -> u64 {
        self.network_received
    }

    pub fn network_transmitted_bytes(
        &self,
    ) -> u64 {
        self.network_transmitted
    }

    pub fn network_received_text(
        &self,
    ) -> String {
        format_rate(
            self.network_received
        )
    }

    pub fn network_transmitted_text(
        &self,
    ) -> String {
        format_rate(self.network_transmitted)
    }

    pub fn memory_percentage(
        &self,
    ) -> f32 {
        if self.total_memory == 0 {
            return 0.0;
        }

        (
            self.used_memory as f64 / self.total_memory as f64 * 100.0
        ) as f32
    }

    pub fn used_memory_gb(
        &self,
    ) -> f64 {
        self.used_memory as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn total_memory_gb(
        &self,
    ) -> f64 {
        self.total_memory as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn formatted_uptime(
        &self,
    ) -> String {
        let total_seconds = self.uptime_seconds;
        let days = total_seconds / 86_400;
        let hours = (total_seconds % 86_400) / 3_600;
        let minutes = (total_seconds % 3_600) / 60;

        if days > 0 {
            format!(
                "{}d {}h {}m",
                days,
                hours,
                minutes,
            )
        } else if hours > 0 {
            format!(
                "{}h {}m",
                hours,
                minutes,
            )
        } else {
            format!(
                "{}m",
                minutes,
            )
        }
    }

    pub fn usage_bar(
        percentage: f32,
        width: usize,
    ) -> String {
        let percentage = percentage.clamp(
            0.0,
            100.0,
        );

        let filled = (percentage / 100.0 * width as f32).round() as usize;
        let empty = width.saturating_sub(filled);

        format!(
            "{}{}",
            "█".repeat(filled),
            "░".repeat(empty),
        )
    }

    pub fn process_count(
        &self,
    ) -> usize {
        self.process_count
    }

    pub fn processes(
        &self,
    ) -> &[ProcessInfo] {
        &self.processes
    }

    pub fn visible_processes(
        &self,
    ) -> &[ProcessInfo] {
        let start = self.process_scroll.min(self.processes.len());
        let end = (start + VISIBLE_PROCESS_ROWS).min(self.processes.len());
        &self.processes[start..end]
    }

    pub fn process_scroll(
        &self,
    ) -> usize {
        self.process_scroll
    }

    fn sort_processes(
        &mut self,
    ) {
        match self.process_sort {
            ProcessSort::Cpu => {
                self.processes.sort_by(
                    |a, b| {
                        b.cpu_usage()
                            .partial_cmp(&a.cpu_usage())
                            .unwrap_or(
                                std::cmp::Ordering::Equal
                            )
                    }
                );
            }

            ProcessSort::Memory => {
                self.processes.sort_by(
                    |a, b| {
                        b.memory()
                            .cmp(&a.memory())
                    }
                );
            }

            ProcessSort::Name => {
                self.processes.sort_by(
                    |a, b| {
                        a.name()
                            .to_lowercase()
                            .cmp(
                                &b.name().to_lowercase()
                            )
                    }
                );
            }
        }
    }

    pub fn process_sort(
        &self,
    ) -> ProcessSort {
        self.process_sort
    }

    pub fn sort_by_cpu(
        &mut self,
    ) {
        self.process_sort = ProcessSort::Cpu;
        self.sort_processes();
        self.keep_selection_visible();
    }

    pub fn sort_by_memory(
        &mut self,
    ) {
        self.process_sort = ProcessSort::Memory;
        self.sort_processes();
        self.keep_selection_visible();
    }

    pub fn sort_by_name(
        &mut self,
    ) {
        self.process_sort = ProcessSort::Name;
        self.sort_processes();
        self.keep_selection_visible();
    }

    pub fn selected_process_index(
        &self,
    ) -> usize {
        let Some(selected_pid) = self.selected_pid
        else {
            return 0;
        };

        self.processes
            .iter()
            .position(
                |process| {
                    process.pid() == selected_pid
                }
            ).unwrap_or(0)
    }

    fn keep_selection_visible(
        &mut self,
    ) {
        if self.processes.is_empty() {
            self.process_scroll = 0;
            return;
        }

        let selected_index = self.selected_process_index();

        if selected_index < self.process_scroll {
            self.process_scroll = selected_index;
        } else if selected_index >= self.process_scroll + VISIBLE_PROCESS_ROWS {
            self.process_scroll = selected_index + 1 - VISIBLE_PROCESS_ROWS;
        }

        let max_scroll = self.processes
                            .len()
                            .saturating_sub(VISIBLE_PROCESS_ROWS);
        
        self.process_scroll = self.process_scroll.min(max_scroll);
    }

    pub fn selected_process(
        &self,
    ) -> Option<&ProcessInfo> {
        let selected_pid = self.selected_pid?;
        self.processes
            .iter()
            .find(
                |process| {
                    process.pid() == selected_pid
                }
            )
    }

    pub fn selected_pid(
        &self,
    ) -> Option<u32> {
        self.selected_pid
    }

    pub fn select_previous_process(
        &mut self,
    ) {
        if self.processes.is_empty() {
            self.selected_pid = None;
            self.process_scroll = 0;
            return;
        }

        let current_index = self.selected_process_index();
        let new_index = if current_index == 0 {
            self.processes.len() - 1 
        } else {
            current_index - 1
        };

        self.selected_pid = Some(
            self.processes[new_index].pid()
        );

        self.keep_selection_visible();
    }

    pub fn select_next_process(
        &mut self,
    ) {
        if self.processes.is_empty() {
            self.selected_pid = None;
            self.process_scroll = 0;
            return;
        }

        let current_index = self.selected_process_index();
        let new_index = (current_index + 1) % self.processes.len();

        self.selected_pid = Some(
            self.processes[new_index].pid()
        );

        self.keep_selection_visible();
    }

    pub fn handle_input(
        &mut self,
        input: PaneInput,
    ) -> bool {
        match self.view {
            SystemMonitorView::Overview => {
                match input {
                    PaneInput::Up => {
                        self.select_previous_process();
                        true
                    }

                    PaneInput::Down => {
                        self.select_next_process();
                        true
                    }

                    PaneInput::Character(character)
                    if character.eq_ignore_ascii_case(&'i') => {
                        self.inspect_selected_process();
                        true
                    }

                    PaneInput::Enter => {
                        self.inspect_selected_process();
                        true
                    }

                    PaneInput::Character(character)
                    if character.eq_ignore_ascii_case(&'c') => {
                        self.sort_by_cpu();
                        true
                    }

                    PaneInput::Character(character)
                    if character.eq_ignore_ascii_case(&'m') => {
                        self.sort_by_memory();
                        true
                    }

                    PaneInput::Character(character)
                    if character.eq_ignore_ascii_case(&'n') => {
                        self.sort_by_name();
                        true
                    }

                    _ => false,
                }
            }

            SystemMonitorView::ProcessInspector => {
                match input {
                    PaneInput::Escape => {
                        self.close_inspector();
                        true
                    }

                    PaneInput::Character(character) 
                    if character.eq_ignore_ascii_case(&'q') => {
                        self.close_inspector();
                        true
                    }

                    PaneInput::Character(character)
                    if character.eq_ignore_ascii_case(&'k') => {
                        self.request_kill_selected_process();
                        true
                    }

                    _ => true,
                }
            }

            SystemMonitorView::ConfirmKill => {
                match input {
                    PaneInput::Escape => {
                        self.cancel_kill();
                        true
                    }

                    PaneInput::Character(character)
                    if character.eq_ignore_ascii_case(&'n') => {
                        self.cancel_kill();
                        true
                    }

                    PaneInput::Character(character) 
                    if character.eq_ignore_ascii_case(&'y') => {
                        self.confirm_kill();
                        true
                    }

                    _ => true,
                }
            }
        }
    }

    pub fn view(
        &self,
    ) -> SystemMonitorView {
        self.view
    }

    pub fn inspect_selected_process(
        &mut self,
    ) {
        if self.selected_process().is_some() {
            self.view = SystemMonitorView::ProcessInspector;
        }
    }

    pub fn close_inspector(
        &mut self,
    ) {
        self.view = SystemMonitorView::Overview;

        if self.selected_process().is_none() {
            self.selected_pid = self.processes
                .first()
                .map(
                    |process| {
                        process.pid()
                    }
                );
        }
    }

    pub fn request_kill_selected_process(
        &mut self,
    ) {
        let Some(process) = self.selected_process()
        else {
            return;
        };

        self.pending_kill_pid = Some(
            process.pid()
        );

        self.view = SystemMonitorView::ConfirmKill;
    }

    pub fn pending_kill_process(
        &self,
    ) -> Option<&ProcessInfo> {
        let pid = self.pending_kill_pid?;

        self.processes
            .iter()
            .find(
                |process| {
                    process.pid() == pid
                }
            )
    }

    pub fn cancel_kill(
        &mut self,
    ) {
        self.pending_kill_pid = None;

        self.view = SystemMonitorView::ProcessInspector;
    }

    pub fn confirm_kill(
        &mut self,
    ) {
        let Some(pid) = self.pending_kill_pid
        else {
            self.view = SystemMonitorView::ProcessInspector;
            return;
        };

        let sysinfo_pid = Pid::from_u32(pid);

        let killed = match self.system.process(sysinfo_pid) {
            Some(process) => {
                let process_name = process
                    .name()
                    .to_string_lossy()
                    .to_string();

                let result = process.kill();
                if result {
                    println!(
                        "Killed process {} (PID {})",
                        process_name,
                        pid
                    );
                } else {
                    println!(
                        "Failed to kill process {} (PID {})",
                        process_name,
                        pid
                    );
                }

                result
            }

            None => {
                println!(
                    "Process PID {} no longer exists",
                    pid
                );

                false
            }
        };

        self.pending_kill_pid = None;
        if killed {
            self.selected_pid = None;
            self.system.refresh_processes(
                sysinfo::ProcessesToUpdate::All,
                true,
            );

            self.last_refresh = Instant::now() - self.refresh_interval;
            self.view = SystemMonitorView::Overview;
        } else {
            self.view = SystemMonitorView::ProcessInspector;
        }
    }
}

fn format_rate(
    bytes: u64,
) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let value = bytes as f64;

    if value >= GB {
        format!(
            "{:.1} GB/s",
            value / GB
        )
    } else if value >= MB {
        format!(
            "{:.1} MB/s",
            value / MB
        )
    } else if value >= KB {
        format!(
            "{:.1} KB/s",
            value / KB
        )
    } else {
        format!(
            "{} B/s",
            bytes
        )
    }
}
