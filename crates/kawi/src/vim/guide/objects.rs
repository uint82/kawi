use kawi_editor::Object;

pub(super) struct ObjectTarget {
    pub(super) key: char,
    pub(super) aliases: &'static [char],
    pub(super) inner_label: &'static str,
    pub(super) around_label: &'static str,
    pub(super) object: Object,
}

pub(super) const OBJECT_TARGETS: &[ObjectTarget] = &[
    object(
        '(',
        &[')'],
        "inner parentheses",
        "around parentheses",
        Object::Paren,
    ),
    object('{', &['}'], "inner braces", "around braces", Object::Brace),
    object(
        '[',
        &[']'],
        "inner brackets",
        "around brackets",
        Object::Bracket,
    ),
    object(
        '<',
        &['>'],
        "inner angle brackets",
        "around angle brackets",
        Object::Angle,
    ),
    object(
        '\'',
        &[],
        "inner single quotes",
        "around single quotes",
        Object::SingleQuote,
    ),
    object(
        '"',
        &[],
        "inner double quotes",
        "around double quotes",
        Object::DoubleQuote,
    ),
    object(
        '`',
        &[],
        "inner backticks",
        "around backticks",
        Object::Backtick,
    ),
    object(
        'p',
        &[],
        "inner paragraph",
        "around paragraph",
        Object::Paragraph,
    ),
];

pub(crate) fn object_target(target: char) -> Option<Object> {
    OBJECT_TARGETS
        .iter()
        .find(|entry| entry.key == target || entry.aliases.contains(&target))
        .map(|entry| entry.object)
}

const fn object(
    key: char,
    aliases: &'static [char],
    inner_label: &'static str,
    around_label: &'static str,
    object: Object,
) -> ObjectTarget {
    ObjectTarget {
        key,
        aliases,
        inner_label,
        around_label,
        object,
    }
}
