use kawi_editor::Action;

use super::{action, aliased, Builtin};
use crate::command::CommandHandler;

pub(super) const COMMANDS: &[Builtin] = &[
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
];
