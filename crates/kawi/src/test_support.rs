const SECTIONS: &[(&str, &[&str])] = &[
    ("base", &["foreground", "background"]),
    ("search_match", &["foreground", "background"]),
    ("search_current", &["foreground", "background"]),
    ("substitute", &["foreground", "background"]),
    ("selection", &["foreground", "background"]),
    ("status", &["foreground", "background"]),
    ("gutter", &["line_number", "non_text"]),
    (
        "git",
        &[
            "add",
            "change",
            "delete",
            "staged_add",
            "staged_change",
            "staged_delete",
        ],
    ),
    ("diagnostics", &["error", "warning", "information", "hint"]),
    (
        "syntax",
        &[
            "comment",
            "keyword",
            "string",
            "number",
            "constant",
            "function",
            "type",
            "variable",
            "property",
            "operator",
            "punctuation",
            "attribute",
            "tag",
            "invalid",
        ],
    ),
];

pub(crate) fn custom_theme() -> String {
    let mut text = String::new();
    for (section, fields) in SECTIONS {
        text.push_str(&format!("[{section}]\n"));
        for field in *fields {
            text.push_str(&format!("{field} = \"#010203\"\n"));
        }
        text.push('\n');
    }
    text
}
