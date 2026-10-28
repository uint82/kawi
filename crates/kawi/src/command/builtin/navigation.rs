use kawi_editor::Action;

use super::{action, Builtin};

pub(super) const COMMANDS: &[Builtin] = &[
    action("cursor.left", Action::MoveLeft, "move left"),
    action("cursor.right", Action::MoveRight, "move right"),
    action("cursor.up", Action::MoveUp, "move up"),
    action("cursor.down", Action::MoveDown, "move down"),
    action(
        "cursor.line_start",
        Action::MoveToLineStart,
        "move to the line start",
    ),
    action(
        "cursor.line_end",
        Action::MoveToLineEnd,
        "move to the line end",
    ),
    action(
        "cursor.first_non_blank",
        Action::MoveToFirstNonBlank,
        "move to the first non-blank",
    ),
    action(
        "cursor.word_next_start",
        Action::MoveToNextWordStart,
        "move to the next word",
    ),
    action(
        "cursor.word_prev_start",
        Action::MoveToPrevWordStart,
        "move to the previous word",
    ),
    action(
        "cursor.word_next_end",
        Action::MoveToNextWordEnd,
        "move to the next word end",
    ),
    action(
        "cursor.word_prev_end",
        Action::MoveToPrevWordEnd,
        "move to the previous word end",
    ),
    action(
        "cursor.big_word_next_start",
        Action::MoveToNextBigWordStart,
        "move to the next WORD",
    ),
    action(
        "cursor.big_word_prev_start",
        Action::MoveToPrevBigWordStart,
        "move to the previous WORD",
    ),
    action(
        "cursor.big_word_next_end",
        Action::MoveToNextBigWordEnd,
        "move to the next WORD end",
    ),
    action(
        "cursor.big_word_prev_end",
        Action::MoveToPrevBigWordEnd,
        "move to the previous WORD end",
    ),
    action(
        "cursor.sentence_next",
        Action::MoveToNextSentence,
        "move to the next sentence",
    ),
    action(
        "cursor.sentence_previous",
        Action::MoveToPrevSentence,
        "move to the previous sentence",
    ),
    action(
        "cursor.paragraph_next",
        Action::MoveToNextParagraph,
        "move to the next paragraph",
    ),
    action(
        "cursor.paragraph_previous",
        Action::MoveToPrevParagraph,
        "move to the previous paragraph",
    ),
    action(
        "cursor.match",
        Action::MoveToMatch,
        "move to the matching bracket",
    ),
    action(
        "cursor.file_start",
        Action::GotoFirstLine,
        "move to the first line",
    ),
    action(
        "cursor.file_end",
        Action::GotoLastLine,
        "move to the last line",
    ),
    action(
        "cursor.viewport_top",
        Action::MoveToViewportTop,
        "move to the viewport top",
    ),
    action(
        "cursor.viewport_middle",
        Action::MoveToViewportMiddle,
        "move to the viewport middle",
    ),
    action(
        "cursor.viewport_bottom",
        Action::MoveToViewportBottom,
        "move to the viewport bottom",
    ),
    action("scroll.line_up", Action::ScrollLineUp, "scroll one line up"),
    action(
        "scroll.line_down",
        Action::ScrollLineDown,
        "scroll one line down",
    ),
    action(
        "scroll.half_up",
        Action::ScrollHalfUp,
        "scroll half a page up",
    ),
    action(
        "scroll.half_down",
        Action::ScrollHalfDown,
        "scroll half a page down",
    ),
    action("scroll.page_up", Action::ScrollPageUp, "scroll one page up"),
    action(
        "scroll.page_down",
        Action::ScrollPageDown,
        "scroll one page down",
    ),
    action("scroll.left", Action::ScrollLeft, "scroll left"),
    action("scroll.right", Action::ScrollRight, "scroll right"),
    action(
        "scroll.half_left",
        Action::ScrollHalfLeft,
        "scroll half a screen left",
    ),
    action(
        "scroll.half_right",
        Action::ScrollHalfRight,
        "scroll half a screen right",
    ),
    action(
        "scroll.cursor_top",
        Action::ScrollCursorStart,
        "scroll the cursor to the top",
    ),
    action(
        "scroll.cursor_bottom",
        Action::ScrollCursorEnd,
        "scroll the cursor to the bottom",
    ),
    action("jump.older", Action::JumpOlder, "jump backward"),
    action("jump.newer", Action::JumpNewer, "jump forward"),
    action(
        "change.older",
        Action::OlderChange,
        "jump to the older change",
    ),
    action(
        "change.newer",
        Action::NewerChange,
        "jump to the newer change",
    ),
    action("fold.toggle", Action::FoldToggle, "toggle the fold"),
    action("fold.open", Action::FoldOpen, "open the fold"),
    action("fold.close", Action::FoldClose, "close the fold"),
    action("fold.open_all", Action::FoldOpenAll, "open all folds"),
    action("fold.close_all", Action::FoldCloseAll, "close all folds"),
    action("fold.next", Action::FoldNext, "move to the next fold"),
    action(
        "fold.previous",
        Action::FoldPrevious,
        "move to the previous fold",
    ),
    action("fold.create", Action::FoldCreate, "create a fold"),
    action("fold.delete", Action::FoldDelete, "delete the fold"),
    action("search.forward", Action::EnterSearch, "search forward"),
    action(
        "search.backward",
        Action::EnterSearchBackward,
        "search backward",
    ),
    action("search.next", Action::NextMatch, "move to the next match"),
    action(
        "search.previous",
        Action::PrevMatch,
        "move to the previous match",
    ),
    action(
        "search.word_forward",
        Action::SearchWordForward,
        "search the word forward",
    ),
    action(
        "search.word_backward",
        Action::SearchWordBackward,
        "search the word backward",
    ),
];
