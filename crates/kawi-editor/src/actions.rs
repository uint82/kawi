#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveToLineStart,
    MoveToFirstNonBlank,
    ScrollLineUp,
    ScrollLineDown,
    ScrollHalfUp,
    ScrollHalfDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollLeft,
    ScrollRight,
    ScrollHalfLeft,
    ScrollHalfRight,
    ScrollCursorStart,
    ScrollCursorEnd,
    MoveToViewportTop,
    MoveToViewportMiddle,
    MoveToViewportBottom,
    JumpOlder,
    JumpNewer,
    OlderChange,
    NewerChange,
    SetMark(char),
    JumpToMark {
        name: char,
        line_wise: bool,
    },
    JumpToPrevious {
        line_wise: bool,
    },
    DeleteToMark {
        name: char,
        line_wise: bool,
    },
    ChangeToMark {
        name: char,
        line_wise: bool,
    },
    YankToMark {
        name: char,
        line_wise: bool,
    },
    EditAlternate,
    SelectRegister(char),
    InsertRegister(char),
    MoveToMatch,
    SearchWordForward,
    SearchWordBackward,
    SearchBareWordForward,
    SearchBareWordBackward,
    DeleteToMatch,
    YankToMatch,
    ChangeToMatch,
    MoveToNextSentence,
    MoveToPrevSentence,
    MoveToNextParagraph,
    MoveToPrevParagraph,
    DeleteToNextSentence,
    DeleteToPrevSentence,
    DeleteToNextParagraph,
    DeleteToPrevParagraph,
    YankToNextSentence,
    YankToPrevSentence,
    YankToNextParagraph,
    YankToPrevParagraph,
    ChangeToNextSentence,
    ChangeToPrevSentence,
    ChangeToNextParagraph,
    ChangeToPrevParagraph,
    PlaceCenter,
    PlaceTop,
    PlaceBottom,
    MoveToLineEnd,
    MoveToNextWordStart,
    MoveToPrevWordStart,
    MoveToNextWordEnd,
    MoveToPrevWordEnd,
    MoveToNextBigWordStart,
    MoveToPrevBigWordStart,
    MoveToNextBigWordEnd,
    MoveToPrevBigWordEnd,
    MoveToNextLineStart,
    MoveToPrevLineStart,
    InsertChar(char),
    InsertNewline,
    InsertTab,
    InsertShiftRight,
    InsertShiftLeft,
    InsertCopyBelow,
    InsertCopyAbove,
    InsertBackspaceWord,
    InsertBackspaceLine,
    DeleteBackward,
    DeleteForward,
    DeleteLine,
    OpenLineBelow,
    OpenLineAbove,
    ReplaceChar(char),
    Increment,
    Decrement,
    IncrementSequence,
    DecrementSequence,
    DeleteBefore,
    Substitute,
    JoinLines,
    JoinLinesNoSpace,
    DeleteWord,
    ChangeWord,
    DeleteWordBackward,
    DeleteWordEnd,
    DeleteWordEndBackward,
    ChangeWordEnd,
    ChangeWordBackward,
    ChangeWordEndBackward,
    DeleteBigWord,
    DeleteBigWordBackward,
    DeleteBigWordEnd,
    ChangeBigWord,
    ChangeBigWordBackward,
    ChangeBigWordEnd,
    YankWord,
    YankWordBackward,
    YankWordEnd,
    YankBigWord,
    YankBigWordBackward,
    YankBigWordEnd,
    YankWordEndBackward,
    YankBigWordEndBackward,
    DeleteBigWordEndBackward,
    ChangeBigWordEndBackward,
    DeleteInnerWord,
    DeleteAroundWord,
    ChangeInnerWord,
    ChangeAroundWord,
    YankInnerWord,
    YankAroundWord,
    DeleteObject {
        inner: bool,
        object: Object,
    },
    ChangeObject {
        inner: bool,
        object: Object,
    },
    YankObject {
        inner: bool,
        object: Object,
    },
    FindChar {
        target: char,
        forward: bool,
        till: bool,
    },
    RepeatFind {
        target: char,
        forward: bool,
        till: bool,
    },
    DeleteFind {
        target: char,
        forward: bool,
        till: bool,
    },
    DeleteRepeatFind {
        target: char,
        forward: bool,
        till: bool,
    },
    ChangeFind {
        target: char,
        forward: bool,
        till: bool,
    },
    ChangeRepeatFind {
        target: char,
        forward: bool,
        till: bool,
    },
    YankFind {
        target: char,
        forward: bool,
        till: bool,
    },
    YankRepeatFind {
        target: char,
        forward: bool,
        till: bool,
    },
    DeleteSelection,
    YankSelection,
    PasteOverSelection,
    ChangeSelection,
    DeleteLines,
    YankLines,
    ChangeLines,
    GotoLastLine,
    GotoFirstLine,
    DeleteToLastLine,
    DeleteToFirstLine,
    DeleteToViewportTop,
    DeleteToViewportMiddle,
    DeleteToViewportBottom,
    YankToLastLine,
    YankToFirstLine,
    YankToViewportTop,
    YankToViewportMiddle,
    YankToViewportBottom,
    ChangeToLastLine,
    ChangeToFirstLine,
    ChangeToViewportTop,
    ChangeToViewportMiddle,
    ChangeToViewportBottom,
    ShiftRight,
    ShiftLeft,
    Reindent,
    ShiftRightAt {
        motion: ShiftMotion,
    },
    ShiftLeftAt {
        motion: ShiftMotion,
    },
    ReindentAt {
        motion: ShiftMotion,
    },
    ToggleCase,
    Lowercase,
    Uppercase,
    ReselectVisual,
    CancelVisual,
    InsertAtLast,
    InsertAtColumnStart,
    ToggleChar,
    RepeatLastChange,
    ToggleCaseAt {
        motion: CaseMotion,
    },
    LowercaseAt {
        motion: CaseMotion,
    },
    UppercaseAt {
        motion: CaseMotion,
    },
    EnterVisualLine,
    EnterVisualBlock,
    SwapSelectionEnds,
    SwapBlockCorners,
    ReplaceSelection(char),
    SelectInnerWord,
    SelectAroundWord,
    SelectObject {
        inner: bool,
        object: Object,
    },
    YankLine,
    DeleteToEnd,
    YankToEnd,
    DeleteToStart,
    DeleteToFirstNonBlank,
    YankToStart,
    YankToFirstNonBlank,
    ChangeToStart,
    ChangeToFirstNonBlank,
    ChangeLine,
    ChangeToEnd,
    PushCount(u8),
    BeginSequence,
    CancelPending,
    PasteAfter,
    PasteBefore,
    PasteAfterAdvance,
    PasteBeforeAdvance,
    Undo,
    Redo,
    Save,
    Quit,
    EnterInsert,
    EnterReplace,
    ExitInsert,
    Append,
    AppendLineEnd,
    InsertLineStart,
    InsertBeforeSelection,
    InsertAfterSelection,
    CollapseSelection,
    EnterVisual,
    ExitVisual,
    JumpTo(usize, usize),
    EnterCommand,
    CompleteCommand,
    CompleteCommandBackward,
    PushCommandChar(char),
    DeleteCommandChar,
    ExecuteCommand,
    CancelCommand,
    HistoryOlder,
    HistoryNewer,
    StartMacroRecording(char),
    RunMacro(char),
    RepeatLastMacro,
    RepeatRecordedMacro,
    ExecuteLastCommand,
    MacroRegisterError(char),
    EnterSearch,
    EnterSearchBackward,
    PushSearchChar(char),
    DeleteSearchChar,
    ConfirmSearch,
    CancelSearch,
    NextMatch,
    PrevMatch,
    OpenFilePicker,
    OpenGrepPicker,
    OpenGrepWordPicker,
    OpenBufferPicker,
    OpenReplacePicker,
    GitNextHunk,
    GitPrevHunk,
    DiagnosticNext,
    DiagnosticPrev,
    GotoDefinition,
    ShowReferences,
    Hover,
    Rename,
    CodeAction,
    Format,
    GitStageHunk,
    GitUndoStage,
    GitStageBuffer,
    GitPreviewHunk,
    GitDiff,
    Noop,
}

impl Action {
    pub fn resets_wanted_column(self) -> bool {
        !matches!(
            self,
            Self::MoveUp
                | Self::MoveDown
                | Self::ScrollLineUp
                | Self::ScrollLineDown
                | Self::ScrollHalfUp
                | Self::ScrollHalfDown
                | Self::ScrollPageUp
                | Self::ScrollPageDown
                | Self::MoveToViewportTop
                | Self::MoveToViewportMiddle
                | Self::MoveToViewportBottom
                | Self::PlaceCenter
                | Self::PlaceTop
                | Self::PlaceBottom
                | Self::GotoLastLine
                | Self::GotoFirstLine
                | Self::MoveToLineEnd
                | Self::EditAlternate
                | Self::Undo
                | Self::Redo
                | Self::EnterVisual
                | Self::EnterVisualLine
                | Self::EnterVisualBlock
                | Self::ExitVisual
                | Self::CancelVisual
                | Self::ReselectVisual
                | Self::SwapSelectionEnds
                | Self::CollapseSelection
                | Self::SelectRegister(_)
                | Self::PushCount(_)
                | Self::BeginSequence
                | Self::CancelPending
                | Self::EnterCommand
                | Self::CompleteCommand
                | Self::CompleteCommandBackward
                | Self::PushCommandChar(_)
                | Self::DeleteCommandChar
                | Self::ExecuteCommand
                | Self::CancelCommand
                | Self::HistoryOlder
                | Self::HistoryNewer
                | Self::StartMacroRecording(_)
                | Self::RunMacro(_)
                | Self::RepeatLastMacro
                | Self::RepeatRecordedMacro
                | Self::ExecuteLastCommand
                | Self::MacroRegisterError(_)
                | Self::EnterSearch
                | Self::EnterSearchBackward
                | Self::PushSearchChar(_)
                | Self::DeleteSearchChar
                | Self::CancelSearch
                | Self::Save
                | Self::Quit
                | Self::OpenFilePicker
                | Self::OpenGrepPicker
                | Self::OpenGrepWordPicker
                | Self::OpenBufferPicker
                | Self::OpenReplacePicker
                | Self::DiagnosticNext
                | Self::DiagnosticPrev
                | Self::GotoDefinition
                | Self::ShowReferences
                | Self::Hover
                | Self::Rename
                | Self::CodeAction
                | Self::Format
                | Self::Noop
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Object {
    Paren,
    Brace,
    Bracket,
    Angle,
    SingleQuote,
    DoubleQuote,
    Backtick,
    Paragraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMotion {
    WordForward,
    WordBackward,
    WordEnd,
    WordEndBackward,
    LineEnd,
    LineStart,
    Find {
        target: char,
        forward: bool,
        till: bool,
    },
    RepeatFind {
        target: char,
        forward: bool,
        till: bool,
    },
    Object {
        inner: bool,
        object: Object,
    },
    Lines {
        upward: bool,
    },
    OperatorLines,
    GotoLine {
        last: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftMotion {
    WordForward,
    WordBackward,
    WordEnd,
    WordEndBackward,
    LineEnd,
    LineStart,
    Find {
        target: char,
        forward: bool,
        till: bool,
    },
    RepeatFind {
        target: char,
        forward: bool,
        till: bool,
    },
    Object {
        inner: bool,
        object: Object,
    },
    GotoLine {
        last: bool,
    },
    Lines {
        upward: bool,
    },
}

// TODO(helix): shared SelectLine action for helix x line select.
// TODO(helix): shared ChangeSelection action for helix c and vim visual c.
// TODO(personal): third keymap flavor alongside vim and helix.
