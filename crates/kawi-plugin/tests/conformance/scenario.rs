use kawi_plugin::{Capability, EVENT_NAMES, HOST_METHODS};

use super::harness::Runtime;

const EDITOR_READ_UI: &[Capability] = &[Capability::EditorRead, Capability::UiMessage];
const DIAGNOSTICS_READ_UI: &[Capability] = &[Capability::DiagnosticsRead, Capability::UiMessage];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Group {
    Core,
    Parity,
    QueuedRevocation,
    TrapRecovery,
    WasmLimits,
    WasmLoadTime,
    Oversize,
    InputHygiene,
}

impl Group {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Parity => "parity",
            Self::QueuedRevocation => "queued-revocation",
            Self::TrapRecovery => "trap-recovery",
            Self::WasmLimits => "wasm-limits",
            Self::WasmLoadTime => "wasm-load-time",
            Self::Oversize => "oversize",
            Self::InputHygiene => "input-hygiene",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Behavior {
    Echo,
    HostCall,
    MultiCall,
    EditorWrite,
    Trap,
    TrapThenRecover,
    Timeout,
    MemoryLimit,
    TableLimit,
    StackLimit,
    HashMismatch,
    UngrantedImport,
    OverImport,
    OversizeComponent,
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
            Self::Trap => "exit-mid",
            Self::TrapThenRecover => "crash-once",
            Self::Timeout => "hang-invoke",
            Self::MemoryLimit | Self::TableLimit | Self::StackLimit => "normal",
            Self::HashMismatch | Self::OversizeComponent => "normal",
            Self::UngrantedImport | Self::OverImport => "host-call",
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
            Self::Trap | Self::TrapThenRecover => "trap-once.wasm",
            Self::Timeout => "loop.wasm",
            Self::MemoryLimit => "memory-limit.wasm",
            Self::TableLimit => "table-limit.wasm",
            Self::StackLimit => "stack-limit.wasm",
            Self::HashMismatch | Self::OversizeComponent => "echo.wasm",
            Self::UngrantedImport | Self::OverImport => "editor-host.wasm",
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
            Self::Trap | Self::TrapThenRecover => "ping",
            _ => "ping",
        }
    }

    pub(super) fn trap_command(self, runtime: Runtime) -> &'static str {
        match runtime {
            Runtime::Process => "probe",
            Runtime::Wasm => "trap",
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
            Self::Trap
            | Self::TrapThenRecover
            | Self::Timeout
            | Self::MemoryLimit
            | Self::TableLimit
            | Self::StackLimit
            | Self::HashMismatch
            | Self::OverImport
            | Self::OversizeComponent => &[],
            Self::UngrantedImport => &[Capability::EditorRead],
            Self::EventDiagnosticsChanged => DIAGNOSTICS_READ_UI,
            Self::EventBufferChanged
            | Self::EventBufferOpened
            | Self::EventFileOpened
            | Self::EventBufferClosed
            | Self::EventFileSaved
            | Self::EventModeChanged => EDITOR_READ_UI,
        }
    }

    pub(super) fn expects_load_warning(self) -> bool {
        self == Self::HashMismatch
    }

    pub(super) fn oversizes_component(self) -> bool {
        self == Self::OversizeComponent
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
            Self::Echo
            | Self::HostCall
            | Self::MultiCall
            | Self::EditorWrite
            | Self::Trap
            | Self::TrapThenRecover
            | Self::Timeout
            | Self::MemoryLimit
            | Self::TableLimit
            | Self::StackLimit
            | Self::HashMismatch
            | Self::UngrantedImport
            | Self::OverImport
            | Self::OversizeComponent => None,
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
    Trap,
    TrapThenRecover,
    FiveStrike,
    CounterReset,
    RestartWithPendingHostCall,
    QueuedRevocation,
    InvalidPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Expected {
    Success,
    Denied(&'static str),
    Protocol,
    Crashed,
    Timeout,
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
        id: "revocation.queued",
        group: Group::QueuedRevocation,
        behavior: Behavior::EventBufferChanged,
        grants: EDITOR_READ_UI,
        action: Action::QueuedRevocation,
        expected: Expected::Success,
        covers_methods: &[],
        covers_events: &["buffer_changed"],
    },
    Scenario {
        id: "trap.crash",
        group: Group::TrapRecovery,
        behavior: Behavior::Trap,
        grants: &[],
        action: Action::Trap,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "trap.timeout",
        group: Group::TrapRecovery,
        behavior: Behavior::Timeout,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Timeout,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "trap.recovery",
        group: Group::TrapRecovery,
        behavior: Behavior::TrapThenRecover,
        grants: &[],
        action: Action::TrapThenRecover,
        expected: Expected::Success,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "trap.five_strike",
        group: Group::TrapRecovery,
        behavior: Behavior::Trap,
        grants: &[],
        action: Action::FiveStrike,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "trap.counter_reset",
        group: Group::TrapRecovery,
        behavior: Behavior::TrapThenRecover,
        grants: &[],
        action: Action::CounterReset,
        expected: Expected::Success,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "trap.restart_pending_host_call",
        group: Group::TrapRecovery,
        behavior: Behavior::HostCall,
        grants: &[Capability::EditorRead],
        action: Action::RestartWithPendingHostCall,
        expected: Expected::Success,
        covers_methods: &["editor.get_state"],
        covers_events: &[],
    },
    Scenario {
        id: "wasm.limits.linear_memory",
        group: Group::WasmLimits,
        behavior: Behavior::MemoryLimit,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "wasm.limits.table",
        group: Group::WasmLimits,
        behavior: Behavior::TableLimit,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "wasm.limits.stack",
        group: Group::WasmLimits,
        behavior: Behavior::StackLimit,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "wasm.load.hash_mismatch",
        group: Group::WasmLoadTime,
        behavior: Behavior::HashMismatch,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "wasm.load.ungranted_import",
        group: Group::WasmLoadTime,
        behavior: Behavior::UngrantedImport,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "wasm.load.over_import",
        group: Group::WasmLoadTime,
        behavior: Behavior::OverImport,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "wasm.oversize.component",
        group: Group::Oversize,
        behavior: Behavior::OversizeComponent,
        grants: &[],
        action: Action::Invoke,
        expected: Expected::Crashed,
        covers_methods: &[],
        covers_events: &[],
    },
    Scenario {
        id: "hygiene.invalid_position",
        group: Group::InputHygiene,
        behavior: Behavior::EditorWrite,
        grants: &[Capability::EditorWrite],
        action: Action::InvalidPosition,
        expected: Expected::Protocol,
        covers_methods: &["editor.apply_edit"],
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
