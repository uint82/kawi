use crate::capability::Capability;
use crate::protocol::PROCESS_PROTOCOL_MINOR;

// `since` is the protocol minor that introduced the API, not the minor this registry was
// written. docs/help/plugin-protocol.md holds the history these constants encode.
pub(crate) const LEGACY_MINOR: u64 = 0;
pub(crate) const EVENTS_MINOR: u64 = 1;
pub(crate) const DIAGNOSTICS_MINOR: u64 = 2;
pub(crate) const COMMAND_COMPOSITION_MINOR: u64 = 3;
pub(crate) const SETTINGS_MINOR: u64 = 4;
pub(crate) const INTERACTIVE_UI_MINOR: u64 = 5;
pub(crate) const DECORATIONS_MINOR: u64 = 7;
pub(crate) const PROCESS_JOBS_MINOR: u64 = 10;
pub(crate) const PROCESS_POLL_MINOR: u64 = 11;
pub(crate) const PANELS_MINOR: u64 = 12;
pub(crate) const WASM_EDITOR_MINOR: u64 = 13;
pub(crate) const WASM_EXTENSIONS_MINOR: u64 = 14;
pub(crate) const WASM_AUTOCMDS_MINOR: u64 = 15;
pub(crate) const WASM_AUTOMATION_MINOR: u64 = 16;
pub(crate) const WASM_OVERLAYS_MINOR: u64 = 17;
pub(crate) const WASM_TASKS_MINOR: u64 = 18;
pub(crate) const WASM_SCRATCH_BUFFERS_MINOR: u64 = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiStability {
    Stable,
    Experimental,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaturityError {
    DeprecatedWithoutVersion,
    DeprecatedWithoutReplacement,
    DeprecatedBeforeIntroduction,
    ActiveWithDeprecationData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiMaturity {
    pub since: u64,
    pub stability: ApiStability,
    pub deprecated_since: Option<u64>,
    pub replacement: Option<&'static str>,
}

impl ApiMaturity {
    pub const fn stable(since: u64) -> Self {
        Self {
            since,
            stability: ApiStability::Stable,
            deprecated_since: None,
            replacement: None,
        }
    }

    pub const fn experimental(since: u64) -> Self {
        Self {
            since,
            stability: ApiStability::Experimental,
            deprecated_since: None,
            replacement: None,
        }
    }

    pub const fn deprecated(since: u64, deprecated_since: u64, replacement: &'static str) -> Self {
        Self {
            since,
            stability: ApiStability::Deprecated,
            deprecated_since: Some(deprecated_since),
            replacement: Some(replacement),
        }
    }

    pub const fn is_stable(self) -> bool {
        matches!(self.stability, ApiStability::Stable)
    }

    pub const fn is_experimental(self) -> bool {
        matches!(self.stability, ApiStability::Experimental)
    }

    pub const fn is_deprecated(self) -> bool {
        matches!(self.stability, ApiStability::Deprecated)
    }

    pub const fn validate(self) -> Result<(), MaturityError> {
        if !self.is_deprecated() {
            return self.validate_active();
        }
        self.validate_deprecated()
    }

    const fn validate_active(self) -> Result<(), MaturityError> {
        if self.deprecated_since.is_some() || self.replacement.is_some() {
            return Err(MaturityError::ActiveWithDeprecationData);
        }
        Ok(())
    }

    const fn validate_deprecated(self) -> Result<(), MaturityError> {
        let Some(deprecated_since) = self.deprecated_since else {
            return Err(MaturityError::DeprecatedWithoutVersion);
        };
        if self.replacement.is_none() {
            return Err(MaturityError::DeprecatedWithoutReplacement);
        }
        if deprecated_since < self.since {
            return Err(MaturityError::DeprecatedBeforeIntroduction);
        }
        Ok(())
    }
}

pub(crate) struct HostMethod {
    pub(crate) name: &'static str,
    pub(crate) maturity: ApiMaturity,
    pub(crate) capability: Option<Capability>,
    pub(crate) interactive: bool,
    pub(crate) mutates: bool,
}

pub(crate) struct Event {
    pub(crate) name: &'static str,
    pub(crate) maturity: ApiMaturity,
    pub(crate) capability: Capability,
}

mod registry;

pub(crate) use registry::{EVENT_ENTRIES, EVENT_NAMES, HOST_METHOD_ENTRIES, HOST_METHOD_NAMES};

pub(crate) fn host_method_entry(method: &str) -> Option<&'static HostMethod> {
    HOST_METHOD_ENTRIES
        .binary_search_by(|entry| entry.name.cmp(method))
        .ok()
        .map(|index| &HOST_METHOD_ENTRIES[index])
}

pub(crate) fn event_entry(event: &str) -> Option<&'static Event> {
    EVENT_ENTRIES
        .binary_search_by(|entry| entry.name.cmp(event))
        .ok()
        .map(|index| &EVENT_ENTRIES[index])
}

pub fn host_method_maturity(method: &str) -> Option<ApiMaturity> {
    host_method_entry(method).map(|entry| entry.maturity)
}

pub fn event_maturity(event: &str) -> Option<ApiMaturity> {
    event_entry(event).map(|entry| entry.maturity)
}

pub fn host_method_since(method: &str) -> Option<u64> {
    host_method_maturity(method).map(|maturity| maturity.since)
}

pub fn event_since(event: &str) -> Option<u64> {
    event_maturity(event).map(|maturity| maturity.since)
}

pub fn is_interactive_host_method(method: &str) -> bool {
    host_method_entry(method).is_some_and(|entry| entry.interactive)
}

pub fn host_method_mutates(method: &str) -> bool {
    host_method_entry(method).is_some_and(|entry| entry.mutates)
}

pub(crate) const fn is_process_available(maturity: ApiMaturity) -> bool {
    maturity.since <= PROCESS_PROTOCOL_MINOR
}

#[cfg(test)]
mod tests;
