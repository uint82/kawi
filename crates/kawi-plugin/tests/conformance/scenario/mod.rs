use kawi_plugin::Capability;

use super::harness::Runtime;

pub(crate) const EDITOR_READ_UI: &[Capability] = &[Capability::EditorRead, Capability::UiMessage];
pub(crate) const DIAGNOSTICS_READ_UI: &[Capability] =
    &[Capability::DiagnosticsRead, Capability::UiMessage];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
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
    pub(crate) fn label(self) -> &'static str {
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
pub(crate) enum Behavior {
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
    DiagnosticsLarge,
    AllImports,
    EventWrite,
    MidCodepoint,
    ControlText,
    EventBufferChanged,
    EventBufferOpened,
    EventFileOpened,
    EventBufferClosed,
    EventFileSaved,
    EventModeChanged,
    EventDiagnosticsChanged,
}

impl Behavior {
    pub(crate) fn process_mode(self) -> &'static str {
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
            Self::DiagnosticsLarge => "diagnostics",
            Self::AllImports => "normal",
            Self::EventWrite => "event-write",
            Self::MidCodepoint => "mid-codepoint",
            Self::ControlText => "control-text",
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

    pub(crate) fn wasm_component(self) -> &'static str {
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
            Self::DiagnosticsLarge => "diagnostics-large.wasm",
            Self::AllImports => "all-imports.wasm",
            Self::EventWrite => "events-write.wasm",
            Self::MidCodepoint => "editor-write-mid.wasm",
            Self::ControlText => "control-text.wasm",
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

    pub(crate) fn command(self) -> &'static str {
        match self {
            Self::HostCall => "read",
            Self::DiagnosticsLarge => "diagnostics",
            Self::ControlText => "control",
            Self::Trap | Self::TrapThenRecover => "ping",
            _ => "ping",
        }
    }

    pub(crate) fn trap_command(self, runtime: Runtime) -> &'static str {
        match runtime {
            Runtime::Process => "probe",
            Runtime::Wasm => "trap",
        }
    }

    pub(crate) fn process_command(self) -> &'static str {
        match self {
            Self::Echo => "ping",
            Self::DiagnosticsLarge => "diagnostics",
            _ => "probe",
        }
    }

    pub(crate) fn requests(self) -> &'static [Capability] {
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
            Self::DiagnosticsLarge => &[Capability::DiagnosticsPublish],
            Self::AllImports => &[
                Capability::EditorRead,
                Capability::EditorWrite,
                Capability::UiMessage,
                Capability::DiagnosticsPublish,
            ],
            Self::EventWrite => &[Capability::EditorRead, Capability::EditorWrite],
            Self::MidCodepoint => &[Capability::EditorWrite],
            Self::ControlText => &[],
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

    pub(crate) fn expects_load_warning(self) -> bool {
        self == Self::HashMismatch
    }

    pub(crate) fn oversizes_component(self) -> bool {
        self == Self::OversizeComponent
    }

    pub(crate) fn event(self) -> Option<&'static str> {
        match self {
            Self::EventBufferChanged => Some("buffer_changed"),
            Self::EventBufferOpened => Some("buffer_opened"),
            Self::EventFileOpened => Some("file_opened"),
            Self::EventBufferClosed => Some("buffer_closed"),
            Self::EventFileSaved => Some("file_saved"),
            Self::EventModeChanged => Some("mode_changed"),
            Self::EventDiagnosticsChanged => Some("diagnostics_changed"),
            Self::EventWrite => Some("buffer_changed"),
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
            | Self::OversizeComponent
            | Self::DiagnosticsLarge
            | Self::AllImports
            | Self::MidCodepoint
            | Self::ControlText => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
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
    QueuedRevocationMidWrite,
    InvalidPosition,
    OversizeInvocation,
    OversizeEvent,
    OversizeHostResponse,
    OversizeDiagnostics,
    AdvertisedParity,
    ControlText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Expected {
    Success,
    Denied(&'static str),
    Protocol,
    Crashed,
    Timeout,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Scenario {
    pub(crate) id: &'static str,
    pub(crate) group: Group,
    pub(crate) behavior: Behavior,
    pub(crate) grants: &'static [Capability],
    pub(crate) action: Action,
    pub(crate) expected: Expected,
    pub(crate) covers_methods: &'static [&'static str],
    pub(crate) covers_events: &'static [&'static str],
}

const DEFAULT_TIMEOUT_MS: u64 = 2_000;
const HEAVY_TIMEOUT_MS: u64 = 10_000;

impl Scenario {
    pub(crate) fn timeout_ms(self) -> u64 {
        if matches!(
            self.action,
            Action::OversizeHostResponse | Action::OversizeInvocation | Action::OversizeDiagnostics
        ) {
            HEAVY_TIMEOUT_MS
        } else {
            DEFAULT_TIMEOUT_MS
        }
    }
}

mod coverage;
mod table;

pub(crate) use coverage::{
    all_events, all_methods, covered_events, covered_methods, Uncovered, UNCOVERED_EVENTS,
    UNCOVERED_METHODS,
};
pub(crate) use table::scenarios;
