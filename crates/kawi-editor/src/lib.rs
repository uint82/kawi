pub mod actions;
pub mod command;
pub mod cursor;
pub mod editing;
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
mod folds;
mod jumps;
mod marks;
mod paragraphs;
mod registers;
mod sentences;

pub use actions::{Action, CaseMotion, Object, ShiftMotion};
pub use command::{
    parse, Command, CommandLine, DebugCommand, GitCommand, LspCommand, PickerCommand,
    QuickfixCommand, SetValue, TaskCommand, UndoCommand,
};
pub use cursor::Cursor;
pub use editing::{
    default_match_pairs, default_pairs, BlockComment, CommentTokens, LanguageEditing, Pair,
    DEFAULT_AUTO_CLOSE_BEFORE,
};
pub use editor::{
    BufferChanges, BufferError, BufferInfo, BufferOptionDefaults, EditError, Editor,
    HistoryEntries, IndentOptions, IndentRules, LanguageIndent, PathAdoption, ViewState,
    WindowOptions, WriteOutcome, DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT,
};
pub use folds::{Fold, FoldMethod, FoldSnapshot, FoldState, DEFAULT_FOLD_COLUMN, MAX_FOLD_COLUMN};
pub use mode::Mode;
pub use objects::bracket_match::{BracketMatch, HIGHLIGHT_MATCH_STEP_BUDGET};
pub use register::{Register, RegisterKind};
pub use search::{MatchSpan, SearchOutcome};
pub use selection::Selection;
pub use viewport::{Viewport, FALLBACK_HEIGHT, FALLBACK_WIDTH};
