#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveToLineStart,
    MoveToLineEnd,
    InsertChar(char),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    Save,
    Quit,
    EnterInsert,
    ExitInsert,
    Noop,
}
