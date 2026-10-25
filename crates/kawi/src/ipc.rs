#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::{
    editor_command, terminal_editor_command, wait, Connection, Event, Server, WaitOutcome,
};
