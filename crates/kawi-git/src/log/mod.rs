mod diff;
mod parser;
mod stream;

pub use diff::{commit_files, commit_patch, file_diff, CommitFile};
pub use parser::{LogEntry, LogOptions};
pub use stream::{LogEvent, LogStream};

#[cfg(test)]
mod tests;
