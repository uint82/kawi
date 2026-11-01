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
