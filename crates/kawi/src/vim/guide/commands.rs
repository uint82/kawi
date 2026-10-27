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
    pub(super) forward_label: &'static str,
    pub(super) backward_label: &'static str,
    pub(super) forward: Action,
    pub(super) backward: Action,
}

pub(super) const FOLD_COMMANDS: &[KeyCommand] = &[
    command('a', "toggle fold", Some(Action::FoldToggle)),
    command('o', "open fold", Some(Action::FoldOpen)),
    command('c', "close fold", Some(Action::FoldClose)),
    command('R', "open all folds", Some(Action::FoldOpenAll)),
    command('M', "close all folds", Some(Action::FoldCloseAll)),
    command('j', "next fold", Some(Action::FoldNext)),
    command('k', "previous fold", Some(Action::FoldPrevious)),
    command('d', "delete fold", Some(Action::FoldDelete)),
];

pub(super) const PLACE_COMMANDS: &[KeyCommand] = &[
    command('z', "center cursor", Some(Action::PlaceCenter)),
    command('t', "cursor to top", Some(Action::PlaceTop)),
    command('b', "cursor to bottom", Some(Action::PlaceBottom)),
    command('h', "scroll left", Some(Action::ScrollLeft)),
    command('l', "scroll right", Some(Action::ScrollRight)),
    command('H', "scroll half page left", Some(Action::ScrollHalfLeft)),
    command('L', "scroll half page right", Some(Action::ScrollHalfRight)),
    command(
        's',
        "scroll cursor to start",
        Some(Action::ScrollCursorStart),
    ),
    command('e', "scroll cursor to end", Some(Action::ScrollCursorEnd)),
];

pub(super) const G_COMMANDS: &[KeyCommand] = &[
    command('g', "go to first line", Some(Action::GotoFirstLine)),
    command('d', "go to definition", Some(Action::GotoDefinition)),
    command('v', "reselect last visual", Some(Action::ReselectVisual)),
    command(
        'J',
        "join lines without space",
        Some(Action::JoinLinesNoSpace),
    ),
    command('-', "older undo state", Some(Action::UndoOlder)),
    command('+', "newer undo state", Some(Action::UndoNewer)),
    command(';', "older change", Some(Action::OlderChange)),
    command(',', "newer change", Some(Action::NewerChange)),
    command('p', "paste after cursor", Some(Action::PasteAfterAdvance)),
    command('P', "paste before cursor", Some(Action::PasteBeforeAdvance)),
    command('i', "insert at last position", Some(Action::InsertAtLast)),
    command(
        'I',
        "insert at column start",
        Some(Action::InsertAtColumnStart),
    ),
    command(
        'e',
        "move to previous word end",
        Some(Action::MoveToPrevWordEnd),
    ),
    command('r', "show references", Some(Action::ShowReferences)),
    command(
        'E',
        "move to previous big word end",
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
    command('j', "move down", Some(Action::MoveDown)),
    command('k', "move up", Some(Action::MoveUp)),
    command('0', "move to start of line", Some(Action::MoveToLineStart)),
    command('$', "move to end of line", Some(Action::MoveToLineEnd)),
    command('~', "toggle case operator", None),
    command('u', "lowercase operator", None),
    command('U', "uppercase operator", None),
    command('c', "comment operator", None),
];

pub(super) const VISUAL_G_COMMANDS: &[KeyCommand] = &[
    command('g', "go to first line", Some(Action::GotoFirstLine)),
    command('v', "reselect last visual", Some(Action::ReselectVisual)),
    command(
        'e',
        "move to previous word end",
        Some(Action::MoveToPrevWordEnd),
    ),
    command(
        'E',
        "move to previous big word end",
        Some(Action::MoveToPrevBigWordEnd),
    ),
    command('c', "comment lines", Some(Action::CommentLines)),
    command('j', "move down", Some(Action::MoveDown)),
    command('k', "move up", Some(Action::MoveUp)),
    command('0', "move to start of line", Some(Action::MoveToLineStart)),
    command('$', "move to end of line", Some(Action::MoveToLineEnd)),
    command('~', "toggle case", Some(Action::ToggleCase)),
    command('u', "lowercase", Some(Action::Lowercase)),
    command('U', "uppercase", Some(Action::Uppercase)),
];

pub(super) const BRACKET_COMMANDS: &[BracketCommand] = &[
    bracket(
        'c',
        "next hunk",
        "previous hunk",
        Action::GitNextHunk,
        Action::GitPrevHunk,
    ),
    bracket(
        'd',
        "next diagnostic",
        "previous diagnostic",
        Action::DiagnosticNext,
        Action::DiagnosticPrev,
    ),
    bracket(
        'q',
        "next quickfix",
        "previous quickfix",
        Action::QuickfixNext,
        Action::QuickfixPrev,
    ),
];

pub(super) const CASE_MOTIONS: &[CommandLabel] = &[
    label('w', "word"),
    label('b', "word backward"),
    label('e', "word end"),
    label('0', "to start of line"),
    label('$', "to end of line"),
    label('j', "down a line"),
    label('k', "up a line"),
    label('G', "to last line"),
    label('g', "more motions"),
    label('f', "find character"),
    label('F', "find character backward"),
    label('t', "till character"),
    label('T', "till character backward"),
    label('i', "inner text object"),
    label('a', "around text object"),
    label(';', "repeat find"),
    label(',', "repeat find backward"),
    label('~', "current line"),
    label('u', "current line"),
    label('U', "current line"),
];

pub(super) const COMMENT_MOTIONS: &[CommandLabel] = &[
    label('c', "current line"),
    label('w', "word"),
    label('b', "word backward"),
    label('e', "word end"),
    label('$', "to end of line"),
    label('0', "to start of line"),
    label('j', "down a line"),
    label('k', "up a line"),
    label('G', "to last line"),
    label('g', "more motions"),
    label('f', "find character"),
    label('F', "find character backward"),
    label('t', "till character"),
    label('T', "till character backward"),
    label('i', "inner text object"),
    label('a', "around text object"),
    label(';', "repeat find"),
    label(',', "repeat find backward"),
];

pub(super) const OPERATOR_G_MOTIONS: &[CommandLabel] = &[
    label('g', "go to first line"),
    label('e', "previous word end"),
    label('E', "previous big word end"),
];

pub(super) const CASE_G_MOTIONS: &[CommandLabel] = &[
    label('g', "go to first line"),
    label('e', "previous word end"),
];

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
    forward_label: &'static str,
    backward_label: &'static str,
    forward: Action,
    backward: Action,
) -> BracketCommand {
    BracketCommand {
        key,
        forward_label,
        backward_label,
        forward,
        backward,
    }
}
