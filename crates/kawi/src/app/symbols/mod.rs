mod apply;
mod cache;
mod events;
mod preview;
mod resolve;
mod rows;
mod session;
mod ui;

pub(crate) use cache::SymbolCache;
pub(crate) use events::handle_event;
pub(crate) use session::SymbolSession;

use std::path::PathBuf;
use std::time::Duration;

const WORKSPACE_DEBOUNCE: Duration = Duration::from_millis(120);
const RESOLVING: &str = "Resolving symbol...";
const NO_LOCATION: &str = "Symbol has no location";
const PREVIEW_LOADING: &str = "Loading preview...";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SymbolMode {
    Document,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SymbolJump {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) column: usize,
}

pub(super) enum SymbolKeyOutcome {
    Consumed,
    Jump(SymbolJump),
    Closed,
}
