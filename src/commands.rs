#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsterCommand {
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,

    SplitVertical,
    SplitHorizontal,
    ClosePane,
    ZoomPane,

    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,

    SystemMonitor,

    Search,
    Copy,
    Paste,

    Shortcuts,
    CommandPalette,

    ReloadConfig,
}

impl AsterCommand {
    pub fn name(
        self,
    ) -> &'static str {
        match self {
            Self::NewTab => "New Tab",
            Self::CloseTab => "Close Tab",
            Self::NextTab => "Next Tab",
            Self::PreviousTab => "Previous Tab",

            Self::SplitVertical => "Split Vertical",
            Self::SplitHorizontal => "Split Horizontal",
            Self::ClosePane => "Close Pane",
            Self::ZoomPane => "Zoom Pane",

            Self::FocusLeft => "Focus Left",
            Self::FocusRight => "Focus Right",
            Self::FocusUp => "Focus Up",
            Self::FocusDown => "Focus Down",

            Self::SystemMonitor => "System Monitor",

            Self::Search => "Search",
            Self::Copy => "Copy",
            Self::Paste => "Paste",

            Self::Shortcuts => "Shortcuts",
            Self::CommandPalette => "CommandPalette",

            Self::ReloadConfig => "Reload Config",
        }
    }

    pub fn command_name(
        self,
    ) -> &'static str {
        match self {
            Self::NewTab => "new-tab",
            Self::CloseTab => "close-tab",
            Self::NextTab => "next-tab",
            Self::PreviousTab => "previous-tab",

            Self::SplitVertical => "split-right",
            Self::SplitHorizontal => "split-down",
            Self::ClosePane => "close-pane",
            Self::ZoomPane => "zoom-pane",

            Self::FocusLeft => "focus-left",
            Self::FocusRight => "focus-right",
            Self::FocusUp => "focus-up",
            Self::FocusDown => "focus-down",

            Self::SystemMonitor => "monitor",

            Self::Search => "search",
            Self::Copy => "copy",
            Self::Paste => "paste",

            Self::Shortcuts => "shortcuts",
            Self::CommandPalette => "commands",

            Self::ReloadConfig => "reload-config",
        }
    }

    pub fn description(
        self,
    ) -> &'static str {
        match self {
            Self::NewTab => "Open a new terminal tab",
            Self::CloseTab => "Close the current tab",
            Self::NextTab => "Moves to the next tab",
            Self::PreviousTab => "Moves to the previous tab",

            Self::SplitVertical => "Split the focused pane vertically",
            Self::SplitHorizontal => "Split the focused pane horizontally",
            Self::ClosePane => "Closes the focused pane",
            Self::ZoomPane => "Zooms current pane",

            Self::FocusLeft => "Focuses the pane to left",
            Self::FocusRight => "Focuses the pane to right",
            Self::FocusUp => "Focuses the pane up",
            Self::FocusDown => "Focuses the pane down",

            Self::SystemMonitor => "Opens the live system monitor",
            Self::Search => "Searches terminal scrollback",
            Self::Copy => "Copies the selected text",
            Self::Paste => "Pastes the text",

            Self::Shortcuts => "Show Aster keyboard shortcuts",
            Self::CommandPalette => "Opens the Aster command palette",
            Self::ReloadConfig => "Reload Aster config",
        }
    }

    pub fn from_command_name(
        name: &str,
    ) -> Option<Self> {
        let normalized = name
            .trim()
            .trim_start_matches(':')
            .to_ascii_lowercase();

        match normalized.as_str() {
            "new-tab" | "tab" => {
                Some(Self::NewTab)
            }

            "close-tab" => {
                Some(Self::CloseTab)
            }

            "next-tab" => {
                Some(Self::NextTab)
            }

            "previous-tab" | "prev-tab" => {
                Some(Self::PreviousTab)
            }

            "split-right" | "split-vertical" => {
                Some(Self::SplitVertical)
            }

            "split-down" | "split-horizontal" => {
                Some(Self::SplitHorizontal)
            }

            "close-pane" => {
                Some(Self::ClosePane)
            }

            "zoom-pane" | "zoom" => {
                Some(Self::ZoomPane)
            }

            "focus-left" => {
                Some(Self::FocusLeft)
            }

            "focus-right" => {
                Some(Self::FocusRight)
            }

            "focus-up" => {
                Some(Self::FocusUp)
            }

            "focus-down" => {
                Some(Self::FocusDown)
            }

            "monitor" | "system-monitor" => {
                Some(Self::SystemMonitor)
            }

            "search" => {
                Some(Self::Search)
            }

            "copy" => {
                Some(Self::Copy)
            }

            "paste" => {
                Some(Self::Paste)
            }

            "shortcuts" | "keys" => {
                Some(Self::Shortcuts)
            }

            "commands" | "palette" => {
                Some(Self::CommandPalette)
            }

            "reload-config" | "reload" => {
                Some(Self::ReloadConfig)
            }

            _ => None,
        }
    }

    pub const ALL: &'static [AsterCommand] = &[
        AsterCommand::NewTab,
        AsterCommand::CloseTab,
        AsterCommand::NextTab,
        AsterCommand::PreviousTab,

        AsterCommand::SplitVertical,
        AsterCommand::SplitHorizontal,
        AsterCommand::ClosePane,
        AsterCommand::ZoomPane,

        AsterCommand::FocusLeft,
        AsterCommand::FocusRight,
        AsterCommand::FocusUp,
        AsterCommand::FocusDown,

        AsterCommand::SystemMonitor,

        AsterCommand::Search,
        AsterCommand::Copy,
        AsterCommand::Paste,

        AsterCommand::Shortcuts,
        AsterCommand::CommandPalette,

        AsterCommand::ReloadConfig,
    ];
}