use kawi_editor::Action;

use super::CommandHandler;

pub(crate) struct Builtin {
    pub(crate) name: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) description: &'static str,
    pub(crate) handler: CommandHandler,
}

const fn action(name: &'static str, action: Action, description: &'static str) -> Builtin {
    Builtin {
        name,
        aliases: &[],
        description,
        handler: CommandHandler::Action(action),
    }
}

const fn aliased(
    name: &'static str,
    aliases: &'static [&'static str],
    action: Action,
    description: &'static str,
) -> Builtin {
    Builtin {
        name,
        aliases,
        description,
        handler: CommandHandler::Action(action),
    }
}

pub(crate) const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "none",
        aliases: &["noop"],
        description: "do nothing",
        handler: CommandHandler::Noop,
    },
    Builtin {
        name: "keys.send",
        aliases: &["send_keys"],
        description: "send a key sequence",
        handler: CommandHandler::SendKeys,
    },
    aliased("file.save", &["save"], Action::Save, "save the buffer"),
    aliased("file.quit", &["quit"], Action::Quit, "quit the editor"),
    action(
        "file.alternate",
        Action::EditAlternate,
        "edit the alternate file",
    ),
    action("mode.insert", Action::EnterInsert, "enter insert mode"),
    action("mode.append", Action::Append, "append after the cursor"),
    action("mode.normal", Action::ExitInsert, "return to normal mode"),
    action("mode.replace", Action::EnterReplace, "enter replace mode"),
    action("mode.visual", Action::EnterVisual, "enter visual mode"),
    action(
        "mode.visual_line",
        Action::EnterVisualLine,
        "enter visual line mode",
    ),
    action(
        "mode.visual_block",
        Action::EnterVisualBlock,
        "enter visual block mode",
    ),
    aliased("edit.undo", &["undo"], Action::Undo, "undo the last change"),
    aliased("edit.redo", &["redo"], Action::Redo, "redo the last change"),
    action(
        "edit.repeat",
        Action::RepeatLastChange,
        "repeat the last change",
    ),
    action(
        "edit.delete_line",
        Action::DeleteLine,
        "delete the current line",
    ),
    action(
        "edit.delete",
        Action::DeleteSelection,
        "delete the selection",
    ),
    action(
        "edit.change",
        Action::ChangeSelection,
        "change the selection",
    ),
    action("edit.yank", Action::YankSelection, "yank the selection"),
    action(
        "edit.paste_after",
        Action::PasteAfter,
        "paste after the cursor",
    ),
    action(
        "edit.paste_before",
        Action::PasteBefore,
        "paste before the cursor",
    ),
    action(
        "edit.delete_word",
        Action::DeleteWord,
        "delete to the next word",
    ),
    action(
        "edit.delete_word_backward",
        Action::DeleteWordBackward,
        "delete the previous word",
    ),
    action(
        "edit.change_word",
        Action::ChangeWord,
        "change to the next word",
    ),
    action("edit.yank_word", Action::YankWord, "yank to the next word"),
    action(
        "edit.delete_to_end",
        Action::DeleteToEnd,
        "delete to the line end",
    ),
    action(
        "edit.delete_to_start",
        Action::DeleteToStart,
        "delete to the line start",
    ),
    action(
        "edit.yank_to_end",
        Action::YankToEnd,
        "yank to the line end",
    ),
    action(
        "edit.yank_to_start",
        Action::YankToStart,
        "yank to the line start",
    ),
    action(
        "edit.change_to_end",
        Action::ChangeToEnd,
        "change to the line end",
    ),
    action(
        "edit.change_to_start",
        Action::ChangeToStart,
        "change to the line start",
    ),
    action(
        "edit.open_below",
        Action::OpenLineBelow,
        "open a line below",
    ),
    action(
        "edit.open_above",
        Action::OpenLineAbove,
        "open a line above",
    ),
    action("edit.join", Action::JoinLines, "join lines"),
    action("edit.increment", Action::Increment, "increment the number"),
    action("edit.decrement", Action::Decrement, "decrement the number"),
    action("edit.indent", Action::ShiftRight, "indent the selection"),
    action("edit.outdent", Action::ShiftLeft, "outdent the selection"),
    action("edit.reindent", Action::Reindent, "reindent the selection"),
    action(
        "edit.toggle_case",
        Action::ToggleCase,
        "toggle the character case",
    ),
    action(
        "edit.lowercase",
        Action::Lowercase,
        "lowercase the selection",
    ),
    action(
        "edit.uppercase",
        Action::Uppercase,
        "uppercase the selection",
    ),
    action("edit.comment", Action::CommentLines, "toggle line comments"),
    action("edit.format", Action::Format, "format the buffer"),
    action(
        "edit.append_line_end",
        Action::AppendLineEnd,
        "append at the line end",
    ),
    action(
        "edit.insert_line_start",
        Action::InsertLineStart,
        "insert at the line start",
    ),
    action("visual.exit", Action::ExitVisual, "leave visual mode"),
    action("visual.cancel", Action::CancelVisual, "cancel visual mode"),
    action(
        "visual.reselect",
        Action::ReselectVisual,
        "reselect the last visual range",
    ),
    action(
        "visual.swap_ends",
        Action::SwapSelectionEnds,
        "swap the selection ends",
    ),
    action(
        "visual.collapse",
        Action::CollapseSelection,
        "collapse the selection",
    ),
    action(
        "visual.inner_word",
        Action::SelectInnerWord,
        "select the inner word",
    ),
    action(
        "visual.around_word",
        Action::SelectAroundWord,
        "select around the word",
    ),
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
    action(
        "picker.files",
        Action::OpenFilePicker,
        "open the file picker",
    ),
    action(
        "picker.buffers",
        Action::OpenBufferPicker,
        "open the buffer picker",
    ),
    action("picker.grep", Action::OpenGrepPicker, "open live grep"),
    action(
        "picker.grep_word",
        Action::OpenGrepWordPicker,
        "grep the word under the cursor",
    ),
    action(
        "picker.replace",
        Action::OpenReplacePicker,
        "open the replace picker",
    ),
    action(
        "picker.symbols",
        Action::OpenDocumentSymbols,
        "open document symbols",
    ),
    action(
        "picker.workspace_symbols",
        Action::OpenWorkspaceSymbols,
        "open workspace symbols",
    ),
    action(
        "buffer.alternate",
        Action::EditAlternate,
        "open the alternate buffer",
    ),
    action(
        "lsp.definition",
        Action::GotoDefinition,
        "go to the definition",
    ),
    action(
        "lsp.references",
        Action::ShowReferences,
        "show the references",
    ),
    action("lsp.hover", Action::Hover, "show the hover information"),
    action("lsp.rename", Action::Rename, "rename the symbol"),
    action(
        "lsp.code_action",
        Action::CodeAction,
        "show the code actions",
    ),
    action("lsp.format", Action::Format, "format the buffer"),
    action(
        "diagnostic.next",
        Action::DiagnosticNext,
        "move to the next diagnostic",
    ),
    action(
        "diagnostic.previous",
        Action::DiagnosticPrev,
        "move to the previous diagnostic",
    ),
    action(
        "diagnostic.show",
        Action::ShowDiagnostic,
        "show the diagnostic",
    ),
    action(
        "quickfix.next",
        Action::QuickfixNext,
        "move to the next quickfix entry",
    ),
    action(
        "quickfix.previous",
        Action::QuickfixPrev,
        "move to the previous quickfix entry",
    ),
    action(
        "quickfix.open",
        Action::QuickfixOpen,
        "open the quickfix list",
    ),
    action(
        "git.next_hunk",
        Action::GitNextHunk,
        "move to the next hunk",
    ),
    action(
        "git.previous_hunk",
        Action::GitPrevHunk,
        "move to the previous hunk",
    ),
    action("git.stage_hunk", Action::GitStageHunk, "stage the hunk"),
    action("git.undo_stage", Action::GitUndoStage, "unstage the hunk"),
    action(
        "git.stage_buffer",
        Action::GitStageBuffer,
        "stage the buffer",
    ),
    action(
        "git.preview_hunk",
        Action::GitPreviewHunk,
        "preview the hunk",
    ),
    action("git.diff", Action::GitDiff, "show the buffer diff"),
    action("task.list", Action::TaskList, "list the tasks"),
    action("task.rerun", Action::TaskRerun, "rerun the last task"),
    action("task.cancel", Action::TaskCancel, "cancel the task"),
    action("task.output", Action::TaskOutput, "show the task output"),
    action(
        "terminal.toggle",
        Action::ToggleTerminal,
        "toggle the terminal",
    ),
    action(
        "explorer.toggle",
        Action::ToggleExplorer,
        "toggle the explorer",
    ),
    action(
        "explorer.focus",
        Action::FocusExplorer,
        "focus the explorer",
    ),
    action(
        "explorer.reveal",
        Action::RevealInExplorer,
        "reveal the current file",
    ),
    action(
        "explorer.parent",
        Action::OpenParentDirectory,
        "open the parent directory",
    ),
];
