use kawi_editor::Object;

pub(super) struct ObjectTarget {
    pub(super) key: char,
    pub(super) aliases: &'static [char],
    pub(super) label: &'static str,
    pub(super) object: Object,
}

pub(super) const OBJECT_TARGETS: &[ObjectTarget] = &[
    ObjectTarget {
        key: '(',
        aliases: &[')'],
        label: "parentheses",
        object: Object::Paren,
    },
    ObjectTarget {
        key: '{',
        aliases: &['}'],
        label: "braces",
        object: Object::Brace,
    },
    ObjectTarget {
        key: '[',
        aliases: &[']'],
        label: "brackets",
        object: Object::Bracket,
    },
    ObjectTarget {
        key: '<',
        aliases: &['>'],
        label: "angle brackets",
        object: Object::Angle,
    },
    ObjectTarget {
        key: '\'',
        aliases: &[],
        label: "single quotes",
        object: Object::SingleQuote,
    },
    ObjectTarget {
        key: '"',
        aliases: &[],
        label: "double quotes",
        object: Object::DoubleQuote,
    },
    ObjectTarget {
        key: '`',
        aliases: &[],
        label: "backticks",
        object: Object::Backtick,
    },
    ObjectTarget {
        key: 'p',
        aliases: &[],
        label: "paragraph",
        object: Object::Paragraph,
    },
];

pub(crate) fn object_target(target: char) -> Option<Object> {
    OBJECT_TARGETS
        .iter()
        .find(|entry| entry.key == target || entry.aliases.contains(&target))
        .map(|entry| entry.object)
}
