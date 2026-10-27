use kawi_editor::Action;

pub(super) struct Sequence {
    pub(super) keys: &'static str,
    pub(super) label: &'static str,
    pub(super) action: Action,
}

pub(super) const OPERATOR_SEQUENCES: &[Sequence] = &[
    sequence("dd", "line", Action::DeleteLine),
    sequence("dw", "word", Action::DeleteWord),
    sequence("db", "word backward", Action::DeleteWordBackward),
    sequence("de", "word end", Action::DeleteWordEnd),
    sequence("d0", "start of line", Action::DeleteToStart),
    sequence("d^", "first non-blank", Action::DeleteToFirstNonBlank),
    sequence("d$", "end of line", Action::DeleteToEnd),
    sequence("d%", "matching bracket", Action::DeleteToMatch),
    sequence("d(", "previous sentence", Action::DeleteToPrevSentence),
    sequence("d)", "next sentence", Action::DeleteToNextSentence),
    sequence("d{", "previous paragraph", Action::DeleteToPrevParagraph),
    sequence("d}", "next paragraph", Action::DeleteToNextParagraph),
    sequence("dG", "last line", Action::DeleteToLastLine),
    sequence("dgg", "first line", Action::DeleteToFirstLine),
    sequence("dH", "top of viewport", Action::DeleteToViewportTop),
    sequence("dM", "middle of viewport", Action::DeleteToViewportMiddle),
    sequence("dL", "bottom of viewport", Action::DeleteToViewportBottom),
    sequence("dge", "previous word end", Action::DeleteWordEndBackward),
    sequence(">>", "line", Action::ShiftRight),
    sequence("<<", "line", Action::ShiftLeft),
    sequence("==", "line", Action::Reindent),
    sequence("diw", "inner word", Action::DeleteInnerWord),
    sequence("daw", "around word", Action::DeleteAroundWord),
    sequence("cc", "line", Action::ChangeLine),
    sequence("cw", "word", Action::ChangeWord),
    sequence("cb", "word backward", Action::ChangeWordBackward),
    sequence("ce", "word end", Action::ChangeWordEnd),
    sequence("c0", "start of line", Action::ChangeToStart),
    sequence("c^", "first non-blank", Action::ChangeToFirstNonBlank),
    sequence("c$", "end of line", Action::ChangeToEnd),
    sequence("c%", "matching bracket", Action::ChangeToMatch),
    sequence("c(", "previous sentence", Action::ChangeToPrevSentence),
    sequence("c)", "next sentence", Action::ChangeToNextSentence),
    sequence("c{", "previous paragraph", Action::ChangeToPrevParagraph),
    sequence("c}", "next paragraph", Action::ChangeToNextParagraph),
    sequence("cG", "last line", Action::ChangeToLastLine),
    sequence("cgg", "first line", Action::ChangeToFirstLine),
    sequence("cH", "top of viewport", Action::ChangeToViewportTop),
    sequence("cM", "middle of viewport", Action::ChangeToViewportMiddle),
    sequence("cL", "bottom of viewport", Action::ChangeToViewportBottom),
    sequence("cge", "previous word end", Action::ChangeWordEndBackward),
    sequence("ciw", "inner word", Action::ChangeInnerWord),
    sequence("caw", "around word", Action::ChangeAroundWord),
    sequence("yy", "line", Action::YankLine),
    sequence("y0", "start of line", Action::YankToStart),
    sequence("y^", "first non-blank", Action::YankToFirstNonBlank),
    sequence("y$", "end of line", Action::YankToEnd),
    sequence("y%", "matching bracket", Action::YankToMatch),
    sequence("y(", "previous sentence", Action::YankToPrevSentence),
    sequence("y)", "next sentence", Action::YankToNextSentence),
    sequence("y{", "previous paragraph", Action::YankToPrevParagraph),
    sequence("y}", "next paragraph", Action::YankToNextParagraph),
    sequence("yG", "last line", Action::YankToLastLine),
    sequence("ygg", "first line", Action::YankToFirstLine),
    sequence("yH", "top of viewport", Action::YankToViewportTop),
    sequence("yM", "middle of viewport", Action::YankToViewportMiddle),
    sequence("yL", "bottom of viewport", Action::YankToViewportBottom),
    sequence("yge", "previous word end", Action::YankWordEndBackward),
    sequence("yiw", "inner word", Action::YankInnerWord),
    sequence("yaw", "around word", Action::YankAroundWord),
    sequence("yw", "word", Action::YankWord),
    sequence("yb", "word backward", Action::YankWordBackward),
    sequence("ye", "word end", Action::YankWordEnd),
    sequence("yW", "big word", Action::YankBigWord),
    sequence("yB", "big word backward", Action::YankBigWordBackward),
    sequence("yE", "big word end", Action::YankBigWordEnd),
    sequence(
        "ygE",
        "previous big word end",
        Action::YankBigWordEndBackward,
    ),
    sequence("dW", "big word", Action::DeleteBigWord),
    sequence("dB", "big word backward", Action::DeleteBigWordBackward),
    sequence("dE", "big word end", Action::DeleteBigWordEnd),
    sequence(
        "dgE",
        "previous big word end",
        Action::DeleteBigWordEndBackward,
    ),
    sequence("cW", "big word", Action::ChangeBigWord),
    sequence("cB", "big word backward", Action::ChangeBigWordBackward),
    sequence("cE", "big word end", Action::ChangeBigWordEnd),
    sequence(
        "cgE",
        "previous big word end",
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
