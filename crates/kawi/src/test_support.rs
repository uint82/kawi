use kawi_lsp::{Diagnostic, DiagnosticsEvent};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SECTIONS: &[(&str, &[&str])] = &[
    ("base", &["foreground", "background"]),
    ("search_match", &["foreground", "background"]),
    ("search_current", &["foreground", "background"]),
    ("substitute", &["foreground", "background"]),
    ("selection", &["foreground", "background"]),
    ("bracket.match", &["foreground", "background"]),
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

pub(crate) fn diagnostic_event(
    server: &str,
    path: &Path,
    diagnostics: Vec<Diagnostic>,
) -> DiagnosticsEvent {
    DiagnosticsEvent {
        server: server.to_string(),
        identifier: None,
        path: path.to_path_buf(),
        version: None,
        diagnostics,
    }
}

pub(crate) fn temp_dir(prefix: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "kawi_test_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        prefix
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp directory");
    dir
}
