use kawi_editor::Action;

pub(super) struct Sequence {
    pub(super) keys: &'static str,
    pub(super) label: &'static str,
    pub(super) action: Action,
}

pub(super) const OPERATOR_SEQUENCES: &[Sequence] = &[
    sequence("dd", "delete line", Action::DeleteLine),
    sequence("dw", "delete word", Action::DeleteWord),
    sequence("db", "delete word backward", Action::DeleteWordBackward),
    sequence("de", "delete word end", Action::DeleteWordEnd),
    sequence("d0", "delete to start of line", Action::DeleteToStart),
    sequence(
        "d^",
        "delete to first non-blank",
        Action::DeleteToFirstNonBlank,
    ),
    sequence("d$", "delete to end of line", Action::DeleteToEnd),
    sequence("d%", "delete to matching bracket", Action::DeleteToMatch),
    sequence(
        "d(",
        "delete to prev sentence",
        Action::DeleteToPrevSentence,
    ),
    sequence(
        "d)",
        "delete to next sentence",
        Action::DeleteToNextSentence,
    ),
    sequence(
        "d{",
        "delete to prev paragraph",
        Action::DeleteToPrevParagraph,
    ),
    sequence(
        "d}",
        "delete to next paragraph",
        Action::DeleteToNextParagraph,
    ),
    sequence("dG", "delete to last line", Action::DeleteToLastLine),
    sequence("dgg", "delete to first line", Action::DeleteToFirstLine),
    sequence(
        "dH",
        "delete to top of viewport",
        Action::DeleteToViewportTop,
    ),
    sequence(
        "dM",
        "delete to middle of viewport",
        Action::DeleteToViewportMiddle,
    ),
    sequence(
        "dL",
        "delete to bottom of viewport",
        Action::DeleteToViewportBottom,
    ),
    sequence(
        "dge",
        "delete to prev word end",
        Action::DeleteWordEndBackward,
    ),
    sequence(">>", "indent line", Action::ShiftRight),
    sequence("<<", "unindent line", Action::ShiftLeft),
    sequence("==", "reindent line", Action::Reindent),
    sequence("diw", "delete inner word", Action::DeleteInnerWord),
    sequence("daw", "delete around word", Action::DeleteAroundWord),
    sequence("cc", "change line", Action::ChangeLine),
    sequence("cw", "change word", Action::ChangeWord),
    sequence("cb", "change word backward", Action::ChangeWordBackward),
    sequence("ce", "change word end", Action::ChangeWordEnd),
    sequence("c0", "change to start of line", Action::ChangeToStart),
    sequence(
        "c^",
        "change to first non-blank",
        Action::ChangeToFirstNonBlank,
    ),
    sequence("c$", "change to end of line", Action::ChangeToEnd),
    sequence("c%", "change to matching bracket", Action::ChangeToMatch),
    sequence(
        "c(",
        "change to prev sentence",
        Action::ChangeToPrevSentence,
    ),
    sequence(
        "c)",
        "change to next sentence",
        Action::ChangeToNextSentence,
    ),
    sequence(
        "c{",
        "change to prev paragraph",
        Action::ChangeToPrevParagraph,
    ),
    sequence(
        "c}",
        "change to next paragraph",
        Action::ChangeToNextParagraph,
    ),
    sequence("cG", "change to last line", Action::ChangeToLastLine),
    sequence("cgg", "change to first line", Action::ChangeToFirstLine),
    sequence(
        "cH",
        "change to top of viewport",
        Action::ChangeToViewportTop,
    ),
    sequence(
        "cM",
        "change to middle of viewport",
        Action::ChangeToViewportMiddle,
    ),
    sequence(
        "cL",
        "change to bottom of viewport",
        Action::ChangeToViewportBottom,
    ),
    sequence(
        "cge",
        "change to prev word end",
        Action::ChangeWordEndBackward,
    ),
    sequence("ciw", "change inner word", Action::ChangeInnerWord),
    sequence("caw", "change around word", Action::ChangeAroundWord),
    sequence("yy", "yank line", Action::YankLine),
    sequence("y0", "yank to start of line", Action::YankToStart),
    sequence("y^", "yank to first non-blank", Action::YankToFirstNonBlank),
    sequence("y$", "yank to end of line", Action::YankToEnd),
    sequence("y%", "yank to matching bracket", Action::YankToMatch),
    sequence("y(", "yank to prev sentence", Action::YankToPrevSentence),
    sequence("y)", "yank to next sentence", Action::YankToNextSentence),
    sequence("y{", "yank to prev paragraph", Action::YankToPrevParagraph),
    sequence("y}", "yank to next paragraph", Action::YankToNextParagraph),
    sequence("yG", "yank to last line", Action::YankToLastLine),
    sequence("ygg", "yank to first line", Action::YankToFirstLine),
    sequence("yH", "yank to top of viewport", Action::YankToViewportTop),
    sequence(
        "yM",
        "yank to middle of viewport",
        Action::YankToViewportMiddle,
    ),
    sequence(
        "yL",
        "yank to bottom of viewport",
        Action::YankToViewportBottom,
    ),
    sequence("yge", "yank to prev word end", Action::YankWordEndBackward),
    sequence("yiw", "yank inner word", Action::YankInnerWord),
    sequence("yaw", "yank around word", Action::YankAroundWord),
    sequence("yw", "yank word", Action::YankWord),
    sequence("yb", "yank word backward", Action::YankWordBackward),
    sequence("ye", "yank word end", Action::YankWordEnd),
    sequence("yW", "yank big word", Action::YankBigWord),
    sequence("yB", "yank big word backward", Action::YankBigWordBackward),
    sequence("yE", "yank big word end", Action::YankBigWordEnd),
    sequence(
        "ygE",
        "yank to prev big word end",
        Action::YankBigWordEndBackward,
    ),
    sequence("dW", "delete big word", Action::DeleteBigWord),
    sequence(
        "dB",
        "delete big word backward",
        Action::DeleteBigWordBackward,
    ),
    sequence("dE", "delete big word end", Action::DeleteBigWordEnd),
    sequence(
        "dgE",
        "delete to prev big word end",
        Action::DeleteBigWordEndBackward,
    ),
    sequence("cW", "change big word", Action::ChangeBigWord),
    sequence(
        "cB",
        "change big word backward",
        Action::ChangeBigWordBackward,
    ),
    sequence("cE", "change big word end", Action::ChangeBigWordEnd),
    sequence(
        "cgE",
        "change to prev big word end",
        Action::ChangeBigWordEndBackward,
    ),
];

pub(crate) fn sequence_action(sequence: &str) -> Option<Action> {
    OPERATOR_SEQUENCES
        .iter()
        .find(|entry| entry.keys == sequence)
        .map(|entry| entry.action)
}

pub(crate) fn is_sequence_prefix(sequence: &str) -> bool {
    OPERATOR_SEQUENCES
        .iter()
        .any(|entry| entry.keys != sequence && entry.keys.starts_with(sequence))
}

const fn sequence(keys: &'static str, label: &'static str, action: Action) -> Sequence {
    Sequence {
        keys,
        label,
        action,
    }
}
