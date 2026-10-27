use kawi_editor::Action;

pub(super) struct KeyCommand {
    pub(super) key: char,
    pub(super) label: &'static str,
    pub(super) action: Option<Action>,
}

pub(super) struct CommandLabel {
    pub(super) key: char,
    pub(super) label: &'static str,
}

pub(super) struct BracketCommand {
    pub(super) key: char,
    pub(super) label: &'static str,
    pub(super) forward: Action,
    pub(super) backward: Action,
}

pub(super) const FOLD_COMMANDS: &[KeyCommand] = &[
    command('a', "toggle", Some(Action::FoldToggle)),
    command('o', "open", Some(Action::FoldOpen)),
    command('c', "close", Some(Action::FoldClose)),
    command('R', "open all", Some(Action::FoldOpenAll)),
    command('M', "close all", Some(Action::FoldCloseAll)),
    command('j', "next", Some(Action::FoldNext)),
    command('k', "previous", Some(Action::FoldPrevious)),
    command('d', "delete", Some(Action::FoldDelete)),
];

pub(super) const PLACE_COMMANDS: &[KeyCommand] = &[
    command('z', "center", Some(Action::PlaceCenter)),
    command('t', "top", Some(Action::PlaceTop)),
    command('b', "bottom", Some(Action::PlaceBottom)),
    command('h', "scroll left", Some(Action::ScrollLeft)),
    command('l', "scroll right", Some(Action::ScrollRight)),
    command('H', "half left", Some(Action::ScrollHalfLeft)),
    command('L', "half right", Some(Action::ScrollHalfRight)),
    command('s', "cursor start", Some(Action::ScrollCursorStart)),
    command('e', "cursor end", Some(Action::ScrollCursorEnd)),
];

pub(super) const G_COMMANDS: &[KeyCommand] = &[
    command('g', "first line", Some(Action::GotoFirstLine)),
    command('d', "definition", Some(Action::GotoDefinition)),
    command('v', "reselect visual", Some(Action::ReselectVisual)),
    command('J', "join lines", Some(Action::JoinLinesNoSpace)),
    command('-', "older undo", Some(Action::UndoOlder)),
    command('+', "newer undo", Some(Action::UndoNewer)),
    command(';', "older change", Some(Action::OlderChange)),
    command(',', "newer change", Some(Action::NewerChange)),
    command('p', "paste after", Some(Action::PasteAfterAdvance)),
    command('P', "paste before", Some(Action::PasteBeforeAdvance)),
    command('i', "insert at last", Some(Action::InsertAtLast)),
    command(
        'I',
        "insert at column start",
        Some(Action::InsertAtColumnStart),
    ),
    command('e', "previous word end", Some(Action::MoveToPrevWordEnd)),
    command('r', "references", Some(Action::ShowReferences)),
    command(
        'E',
        "previous big word end",
        Some(Action::MoveToPrevBigWordEnd),
    ),
    command(
        '*',
        "search word forward",
        Some(Action::SearchBareWordForward),
    ),
    command(
        '#',
        "search word backward",
        Some(Action::SearchBareWordBackward),
    ),
    command('j', "down", Some(Action::MoveDown)),
    command('k', "up", Some(Action::MoveUp)),
    command('0', "start of line", Some(Action::MoveToLineStart)),
    command('$', "end of line", Some(Action::MoveToLineEnd)),
    command('~', "toggle case", None),
    command('u', "lowercase", None),
    command('U', "uppercase", None),
    command('c', "comment", None),
];

pub(super) const VISUAL_G_COMMANDS: &[KeyCommand] = &[
    command('g', "first line", Some(Action::GotoFirstLine)),
    command('v', "reselect visual", Some(Action::ReselectVisual)),
    command('e', "previous word end", Some(Action::MoveToPrevWordEnd)),
    command(
        'E',
        "previous big word end",
        Some(Action::MoveToPrevBigWordEnd),
    ),
    command('c', "comment", Some(Action::CommentLines)),
    command('j', "down", Some(Action::MoveDown)),
    command('k', "up", Some(Action::MoveUp)),
    command('0', "start of line", Some(Action::MoveToLineStart)),
    command('$', "end of line", Some(Action::MoveToLineEnd)),
    command('~', "toggle case", Some(Action::ToggleCase)),
    command('u', "lowercase", Some(Action::Lowercase)),
    command('U', "uppercase", Some(Action::Uppercase)),
];

pub(super) const BRACKET_COMMANDS: &[BracketCommand] = &[
    bracket('c', "hunk", Action::GitNextHunk, Action::GitPrevHunk),
    bracket(
        'd',
        "diagnostic",
        Action::DiagnosticNext,
        Action::DiagnosticPrev,
    ),
    bracket('q', "quickfix", Action::QuickfixNext, Action::QuickfixPrev),
];

pub(super) const CASE_MOTIONS: &[CommandLabel] = &[
    label('w', "word"),
    label('b', "word backward"),
    label('e', "word end"),
    label('0', "start of line"),
    label('$', "end of line"),
    label('j', "down"),
    label('k', "up"),
    label('G', "last line"),
    label('g', "g motion"),
    label('f', "find character"),
    label('F', "find character backward"),
    label('t', "till character"),
    label('T', "till character backward"),
    label('i', "inner object"),
    label('a', "around object"),
    label(';', "repeat find"),
    label(',', "repeat find backward"),
    label('~', "line"),
    label('u', "line"),
    label('U', "line"),
];

pub(super) const COMMENT_MOTIONS: &[CommandLabel] = &[
    label('c', "line"),
    label('w', "word"),
    label('b', "word backward"),
    label('e', "word end"),
    label('$', "end of line"),
    label('0', "start of line"),
    label('j', "down"),
    label('k', "up"),
    label('G', "last line"),
    label('g', "g motion"),
    label('f', "find character"),
    label('F', "find character backward"),
    label('t', "till character"),
    label('T', "till character backward"),
    label('i', "inner object"),
    label('a', "around object"),
    label(';', "repeat find"),
    label(',', "repeat find backward"),
];

pub(super) const CASE_G_MOTIONS: &[CommandLabel] =
    &[label('g', "first line"), label('e', "previous word end")];

pub(crate) fn fold_action(key: char) -> Option<Action> {
    action_for(FOLD_COMMANDS, key)
}

pub(crate) fn place_action(key: char) -> Option<Action> {
    action_for(PLACE_COMMANDS, key)
}

pub(crate) fn g_action(key: char) -> Option<Action> {
    action_for(G_COMMANDS, key)
}

pub(crate) fn visual_g_action(key: char) -> Option<Action> {
    action_for(VISUAL_G_COMMANDS, key)
}

pub(crate) fn bracket_action(key: char, forward: bool) -> Option<Action> {
    BRACKET_COMMANDS
        .iter()
        .find(|command| command.key == key)
        .map(|command| {
            if forward {
                command.forward
            } else {
                command.backward
            }
        })
}

fn action_for(commands: &[KeyCommand], key: char) -> Option<Action> {
    commands
        .iter()
        .find(|command| command.key == key)
        .and_then(|command| command.action)
}

const fn command(key: char, label: &'static str, action: Option<Action>) -> KeyCommand {
    KeyCommand { key, label, action }
}

const fn label(key: char, label: &'static str) -> CommandLabel {
    CommandLabel { key, label }
}

const fn bracket(
    key: char,
    label: &'static str,
    forward: Action,
    backward: Action,
) -> BracketCommand {
    BracketCommand {
        key,
        label,
        forward,
        backward,
    }
}
