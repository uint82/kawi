mod behavior;
mod network;

use kawi_plugin::Capability;

pub(crate) use behavior::{Behavior, DIAGNOSTICS_READ_UI, EDITOR_READ_UI};
pub(crate) use network::{
    network_case, network_endpoint, NetworkGrant, NetworkTarget, PUBLIC_GRANT,
};
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
pub(crate) use network::{resolver_script, ResolverScript};

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
    Network,
    Authoring,
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
            Self::Network => "network",
            Self::Authoring => "authoring",
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
    MidCodepointPosition,
    NetworkProbe,
    NetworkScripted,
    Trim,
    TrimStale,
    TrimDenied,
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

const TIMEOUT_MS: u64 = 2_000;
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const HEAVY_TIMEOUT_MS: u64 = 10_000;

impl Scenario {
    pub(crate) fn timeout_ms(self) -> u64 {
        if self.behavior == Behavior::Timeout {
            TIMEOUT_MS
        } else if self.behavior == Behavior::NetworkProbeHeavy
            || matches!(
                self.action,
                Action::OversizeHostResponse
                    | Action::OversizeInvocation
                    | Action::OversizeDiagnostics
                    | Action::QueuedRevocation
                    | Action::QueuedRevocationMidWrite
            )
        {
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
