use kawi_plugin::Capability;

use super::super::harness::Runtime;

pub(crate) const EDITOR_READ_UI: &[Capability] = &[Capability::EditorRead, Capability::UiMessage];
pub(crate) const DIAGNOSTICS_READ_UI: &[Capability] =
    &[Capability::DiagnosticsRead, Capability::UiMessage];

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
    NetworkProbe,
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
            Self::NetworkProbe => "normal",
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
            Self::NetworkProbe => "network.wasm",
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
            Self::NetworkProbe => "connect",
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
            Self::EventWrite => &[
                Capability::EditorRead,
                Capability::EditorWrite,
                Capability::UiMessage,
            ],
            Self::MidCodepoint => &[Capability::EditorWrite],
            Self::ControlText => &[Capability::UiMessage, Capability::DiagnosticsPublish],
            Self::NetworkProbe => &[
                Capability::NetConnect,
                Capability::FsRead,
                Capability::FsWrite,
            ],
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
            | Self::ControlText
            | Self::NetworkProbe => None,
        }
    }
}
