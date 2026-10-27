mod config;
mod parser;
mod runner;

pub const PROJECT_TASKS: &str = ".kawi/tasks.toml";

pub use config::{load, TaskLoad, TaskSpec, TaskWarning, Tasks};
pub use parser::{Finding, OutputStream, Parser, ParserError, ParserSpec, RegexParser, Severity};
pub use runner::{TaskEvent, TaskOptions, TaskRunner, TaskStatus};
