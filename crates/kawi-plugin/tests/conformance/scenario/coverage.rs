use kawi_plugin::{EVENT_NAMES, HOST_METHODS};

use super::scenarios;

pub(crate) struct Uncovered {
    pub(crate) name: &'static str,
    pub(crate) reason: &'static str,
}

pub(crate) const UNCOVERED_METHODS: &[Uncovered] = &[
    Uncovered {
        name: "editor.execute",
        reason:
            "requires the editor command registry; covered by app end-to-end process and wasm tests",
    },
    Uncovered {
        name: "editor.list_commands",
        reason:
            "requires the editor command registry; covered by app end-to-end process and wasm tests",
    },
    Uncovered {
        name: "config.get",
        reason: "requires the plugin settings store; covered by app settings and end-to-end tests",
    },
    Uncovered {
        name: "config.list",
        reason: "requires the plugin settings store; covered by app settings and end-to-end tests",
    },
    Uncovered {
        name: "ui.input",
        reason: "requires interactive editor input; covered by app plugin UI tests",
    },
    Uncovered {
        name: "ui.select",
        reason: "requires interactive editor input; covered by app plugin UI tests",
    },
    Uncovered {
        name: "ui.confirm",
        reason: "requires interactive editor input; covered by app plugin UI tests",
    },
    Uncovered {
        name: "ui.progress_start",
        reason: "requires interactive editor input; covered by app plugin UI tests",
    },
    Uncovered {
        name: "ui.progress_update",
        reason: "requires interactive editor input; covered by app plugin UI tests",
    },
    Uncovered {
        name: "ui.progress_finish",
        reason: "requires interactive editor input; covered by app plugin UI tests",
    },
    Uncovered {
        name: "ui.panel_open",
        reason: "requires editor panel state; covered by app process and wasm panel tests",
    },
    Uncovered {
        name: "ui.panel_update",
        reason: "requires editor panel state; covered by app process and wasm panel tests",
    },
    Uncovered {
        name: "ui.panel_focus",
        reason: "requires editor panel state; covered by app process and wasm panel tests",
    },
    Uncovered {
        name: "ui.panel_close",
        reason: "requires editor panel state; covered by app process and wasm panel tests",
    },
    Uncovered {
        name: "editor.set_decorations",
        reason: "requires editor buffer state; covered by app plugin visual tests",
    },
    Uncovered {
        name: "ui.set_statusline",
        reason: "requires editor statusline state; covered by app plugin visual tests",
    },
    Uncovered {
        name: "process.open",
        reason: "requires a live child process; covered by app plugin job tests",
    },
    Uncovered {
        name: "process.write",
        reason: "requires a live child process; covered by app plugin job tests",
    },
    Uncovered {
        name: "process.close_stdin",
        reason: "requires a live child process; covered by app plugin job tests",
    },
    Uncovered {
        name: "process.signal",
        reason: "requires a live child process; covered by app plugin job tests",
    },
    Uncovered {
        name: "process.kill",
        reason: "requires a live child process; covered by app plugin job tests",
    },
    Uncovered {
        name: "process.status",
        reason: "requires a live child process; covered by app plugin job tests",
    },
    Uncovered {
        name: "process.poll",
        reason:
            "requires a live child process and queued WASM events; covered by app WASM job tests",
    },
    Uncovered {
        name: "buffer.list",
        reason: "requires editor buffer state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "buffer.get",
        reason: "requires editor buffer state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "buffer.open",
        reason:
            "requires a workspace and editor buffer state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "buffer.apply_edit",
        reason: "requires editor buffer state and path scope; covered by app plugin host API tests",
    },
    Uncovered {
        name: "window.list",
        reason: "requires editor window state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "window.get",
        reason: "requires editor window state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "window.focus",
        reason: "requires editor window state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "window.open",
        reason: "requires editor window state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "window.close",
        reason: "requires editor window state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "editor.set_cursor",
        reason: "requires editor buffer state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "editor.set_selection",
        reason: "requires editor buffer state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "editor.set_mode",
        reason: "requires editor buffer state; covered by app plugin host API tests",
    },
    Uncovered {
        name: "options.list",
        reason: "requires the editor option registry; covered by app plugin host API tests",
    },
    Uncovered {
        name: "options.get",
        reason: "requires the editor option registry; covered by app plugin host API tests",
    },
];
pub(crate) const UNCOVERED_EVENTS: &[Uncovered] = &[
    Uncovered {
        name: "buffer_enter",
        reason: "requires editor buffer focus transitions; covered by app plugin event tests",
    },
    Uncovered {
        name: "buffer_leave",
        reason: "requires editor buffer focus transitions; covered by app plugin event tests",
    },
    Uncovered {
        name: "window_enter",
        reason: "requires editor window focus transitions; covered by app plugin event tests",
    },
    Uncovered {
        name: "window_leave",
        reason: "requires editor window focus transitions; covered by app plugin event tests",
    },
    Uncovered {
        name: "cursor_moved",
        reason: "requires editor cursor state; covered by app plugin event tests",
    },
    Uncovered {
        name: "selection_changed",
        reason: "requires editor selection state; covered by app plugin event tests",
    },
    Uncovered {
        name: "focus_gained",
        reason: "requires editor focus transitions; covered by app plugin event tests",
    },
    Uncovered {
        name: "focus_lost",
        reason: "requires editor focus transitions; covered by app plugin event tests",
    },
];

pub(crate) fn covered_methods() -> Vec<&'static str> {
    scenarios()
        .iter()
        .flat_map(|scenario| scenario.covers_methods.iter().copied())
        .collect()
}

pub(crate) fn covered_events() -> Vec<&'static str> {
    scenarios()
        .iter()
        .flat_map(|scenario| scenario.covers_events.iter().copied())
        .collect()
}

pub(crate) fn all_methods() -> &'static [&'static str] {
    &HOST_METHODS
}

pub(crate) fn all_events() -> &'static [&'static str] {
    &EVENT_NAMES
}
