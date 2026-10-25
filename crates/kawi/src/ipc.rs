#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::{
    editor_command, terminal_editor_command, wait, Connection, Event, Server, WaitOutcome,
    SOCKET_ENV,
};

#[cfg(test)]
pub(crate) use unix::wait_on;
