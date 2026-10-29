use kawi_plugin::{Capability, EVENT_NAMES, HOST_METHODS};

const EDITOR_READ_UI: &[Capability] = &[Capability::EditorRead, Capability::UiMessage];
const DIAGNOSTICS_READ_UI: &[Capability] = &[Capability::DiagnosticsRead, Capability::UiMessage];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Group {
    Core,
    Parity,
}

impl Group {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Parity => "parity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Behavior {
    Echo,
    HostCall,
    MultiCall,
    EditorWrite,
    EventBufferChanged,
    EventBufferOpened,
    EventFileOpened,
    EventBufferClosed,
    EventFileSaved,
    EventModeChanged,
    EventDiagnosticsChanged,
}

impl Behavior {
    pub(super) fn process_mode(self) -> &'static str {
        match self {
            Self::Echo => "normal",
            Self::HostCall => "host-call",
            Self::MultiCall => "multi-call",
            Self::EditorWrite => "apply-edit",
            Self::EventBufferChanged
            | Self::EventBufferOpened
            | Self::EventFileOpened
            | Self::EventBufferClosed
            | Self::EventFileSaved
            | Self::EventModeChanged
            | Self::EventDiagnosticsChanged => "event-host",
        }
    }

    pub(super) fn wasm_component(self) -> &'static str {
        match self {
            Self::Echo => "echo.wasm",
            Self::HostCall => "editor-host.wasm",
            Self::MultiCall => "multi-import.wasm",
            Self::EditorWrite => "editor-write.wasm",
            Self::EventBufferChanged
            | Self::EventBufferOpened
            | Self::EventFileOpened
            | Self::EventBufferClosed
            | Self::EventFileSaved
            | Self::EventModeChanged
            | Self::EventDiagnosticsChanged => "events.wasm",
        }
    }

    pub(super) fn command(self) -> &'static str {
        match self {
            Self::HostCall => "read",
            _ => "ping",
        }
    }

    pub(super) fn process_command(self) -> &'static str {
        match self {
            Self::Echo => "ping",
            _ => "probe",
        }
    }

    pub(super) fn requests(self) -> &'static [Capability] {
        match self {
            Self::Echo => &[],
            Self::HostCall => &[Capability::EditorRead],
            Self::MultiCall => &[
                Capability::EditorRead,
                Capability::UiMessage,
                Capability::DiagnosticsPublish,
            ],
            Self::EditorWrite => &[Capability::EditorWrite],
            Self::EventDiagnosticsChanged => DIAGNOSTICS_READ_UI,
            Self::EventBufferChanged
            | Self::EventBufferOpened
            | Self::EventFileOpened
            | Self::EventBufferClosed
            | Self::EventFileSaved
            | Self::EventModeChanged => EDITOR_READ_UI,
        }
    }

    pub(super) fn event(self) -> Option<&'static str> {
        match self {
            Self::EventBufferChanged => Some("buffer_changed"),
            Self::EventBufferOpened => Some("buffer_opened"),
            Self::EventFileOpened => Some("file_opened"),
            Self::EventBufferClosed => Some("buffer_closed"),
            Self::EventFileSaved => Some("file_saved"),
            Self::EventModeChanged => Some("mode_changed"),
            Self::EventDiagnosticsChanged => Some("diagnostics_changed"),
            Self::Echo | Self::HostCall | Self::MultiCall | Self::EditorWrite => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Invoke,
    RevokeThenInvoke,
    RevokeThenNotify,
    NotifyThenInvoke,
    StaleRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Expected {
    Success,
    Denied(&'static str),
    Protocol,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Scenario {
    pub(super) id: &'static str,
    pub(super) group: Group,
    pub(super) behavior: Behavior,
    pub(super) grants: &'static [Capability],
    pub(super) action: Action,
    pub(super) expected: Expected,
    pub(super) covers_methods: &'static [&'static str],
    pub(super) covers_events: &'static [&'static str],
}

pub(super) const SCENARIOS: &[Scenario] = &[
    Scenario {
        id: "core.echo",
        group: Group::Core,
        behavior: Behavior::Echo,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Success,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "core.denial",
        group: Group::Core,
        behavior: Behavior::HostCall,
        grants: &[Capability::EditorRead],
        action: Action::RevokeThenInvoke,
        expected: Expected::Denied("editor.read"),
        covers_methods: &["editor.get_state"],
        covers_events: &[],
    },
    Scenario {
        id: "core.revocation",
        group: Group::Core,
        behavior: Behavior::EventBufferChanged,
        grants: EDITOR_READ_UI,
        action: Action::RevokeThenNotify,
        expected: Expected::Denied("editor.read"),
        covers_methods: &[],
        covers_events: &["buffer_changed"],
    },
    Scenario {
        id: "core.stale_revision",
        group: Group::Core,
        behavior: Behavior::EditorWrite,
        grants: &[Capability::EditorWrite],
        action: Action::StaleRevision,
        expected: Expected::Protocol,
        covers_methods: &["editor.apply_edit"],
        covers_events: &[],
    },
    Scenario {
        id: "parity.grants.editor_read",
        group: Group::Parity,
        behavior: Behavior::HostCall,
        grants: &[Capability::EditorRead],
        action: Action::Invoke,
        expected: Expected::Success,
        covers_methods: &["editor.get_state"],
        covers_events: &[],
    },
    Scenario {
        id: "parity.grants.read_write",
        group: Group::Parity,
        behavior: Behavior::HostCall,
        grants: &[Capability::EditorRead, Capability::EditorWrite],
        action: Action::Invoke,
        expected: Expected::Success,
        covers_methods: &["editor.get_state"],
        covers_events: &[],
    },
    Scenario {
        id: "parity.grants.multi",
        group: Group::Parity,
        behavior: Behavior::MultiCall,
        grants: &[
            Capability::EditorRead,
            Capability::UiMessage,
            Capability::DiagnosticsPublish,
        ],
        action: Action::Invoke,
        expected: Expected::Success,
        covers_methods: &["editor.get_state", "editor.set_diagnostics", "ui.message"],
        covers_events: &[],
    },
    Scenario {
        id: "parity.events.buffer_changed",
        group: Group::Parity,
        behavior: Behavior::EventBufferChanged,
        grants: EDITOR_READ_UI,
        action: Action::NotifyThenInvoke,
        expected: Expected::Success,
        covers_methods: &["ui.message"],
        covers_events: &["buffer_changed"],
    },
    Scenario {
        id: "parity.events.buffer_opened",
        group: Group::Parity,
        behavior: Behavior::EventBufferOpened,
        grants: EDITOR_READ_UI,
        action: Action::NotifyThenInvoke,
        expected: Expected::Success,
        covers_methods: &["ui.message"],
        covers_events: &["buffer_opened"],
    },
    Scenario {
        id: "parity.events.file_opened",
        group: Group::Parity,
        behavior: Behavior::EventFileOpened,
        grants: EDITOR_READ_UI,
        action: Action::NotifyThenInvoke,
        expected: Expected::Success,
        covers_methods: &["ui.message"],
        covers_events: &["file_opened"],
    },
    Scenario {
        id: "parity.events.buffer_closed",
        group: Group::Parity,
        behavior: Behavior::EventBufferClosed,
        grants: EDITOR_READ_UI,
        action: Action::NotifyThenInvoke,
        expected: Expected::Success,
        covers_methods: &["ui.message"],
        covers_events: &["buffer_closed"],
    },
    Scenario {
        id: "parity.events.file_saved",
        group: Group::Parity,
        behavior: Behavior::EventFileSaved,
        grants: EDITOR_READ_UI,
        action: Action::NotifyThenInvoke,
        expected: Expected::Success,
        covers_methods: &["ui.message"],
        covers_events: &["file_saved"],
    },
    Scenario {
        id: "parity.events.mode_changed",
        group: Group::Parity,
        behavior: Behavior::EventModeChanged,
        grants: EDITOR_READ_UI,
        action: Action::NotifyThenInvoke,
        expected: Expected::Success,
        covers_methods: &["ui.message"],
        covers_events: &["mode_changed"],
    },
    Scenario {
        id: "parity.events.diagnostics_changed",
        group: Group::Parity,
        behavior: Behavior::EventDiagnosticsChanged,
        grants: DIAGNOSTICS_READ_UI,
        action: Action::NotifyThenInvoke,
        expected: Expected::Success,
        covers_methods: &["ui.message"],
        covers_events: &["diagnostics_changed"],
    },
];

pub(super) struct Uncovered {
    pub(super) name: &'static str,
    pub(super) reason: &'static str,
}

pub(super) const UNCOVERED_METHODS: &[Uncovered] = &[];
pub(super) const UNCOVERED_EVENTS: &[Uncovered] = &[];

pub(super) fn covered_methods() -> Vec<&'static str> {
    SCENARIOS
        .iter()
        .flat_map(|scenario| scenario.covers_methods.iter().copied())
        .collect()
}

pub(super) fn covered_events() -> Vec<&'static str> {
    SCENARIOS
        .iter()
        .flat_map(|scenario| scenario.covers_events.iter().copied())
        .collect()
}

pub(super) fn all_methods() -> &'static [&'static str] {
    &HOST_METHODS
}

pub(super) fn all_events() -> &'static [&'static str] {
    &EVENT_NAMES
}
