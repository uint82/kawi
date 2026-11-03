use crate::LanguageConfig;

mod docs;
mod scripting;
mod systems;
mod web;

pub(super) const C_PAIRS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"')];
pub(super) const C_PAIRS_SINGLE: [(char, char); 5] =
    [('(', ')'), ('[', ']'), ('{', '}'), ('\'', '\''), ('"', '"')];
pub(super) const SCRIPT_PAIRS: [(char, char); 6] = [
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('\'', '\''),
    ('"', '"'),
    ('`', '`'),
];
pub(super) const HTML_PAIRS: [(char, char); 6] = [
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('<', '>'),
    ('\'', '\''),
    ('"', '"'),
];

pub(super) fn languages() -> Vec<LanguageConfig> {
    let mut languages = Vec::new();
    languages.extend(systems::languages());
    languages.extend(web::languages());
    languages.extend(scripting::languages());
    languages.extend(docs::languages());
    languages
}
