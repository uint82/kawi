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
];
pub(crate) const UNCOVERED_EVENTS: &[Uncovered] = &[];

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
