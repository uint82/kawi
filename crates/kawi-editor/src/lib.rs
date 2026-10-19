pub mod actions;
pub mod command;
pub mod cursor;
pub mod editor;
pub mod find;
pub mod matcher;
pub mod mode;
pub mod objects;
pub mod paths;
pub mod register;
pub mod search;
pub mod search_stat;
pub mod selection;
pub mod viewport;
pub mod words;

mod case_map;
pub mod clipboard;
mod jumps;
mod paragraphs;
mod registers;
mod sentences;

pub use actions::{Action, CaseMotion, Object, ShiftMotion};
pub use command::{parse, Command, CommandLine};
pub use cursor::Cursor;
pub use editor::{
    BufferError, EditError, Editor, HistoryEntries, WriteOutcome, DEFAULT_HISTORY_LIMIT,
    MAX_HISTORY_LIMIT,
};
pub use mode::Mode;
pub use register::{Register, RegisterKind};
pub use search::{MatchSpan, SearchOutcome};
pub use selection::Selection;
pub use viewport::{Viewport, FALLBACK_HEIGHT, FALLBACK_WIDTH};
