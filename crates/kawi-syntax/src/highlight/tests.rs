use ropey::Rope;

use super::*;
use crate::limits::{MAX_LINE_BYTES, WINDOW_PARSE_BYTES};
use crate::span::SyntaxKey;
use support::{highlight, name_at, request};

mod bash;
mod c;
mod captures;
mod cpp;
mod css;
mod go;
mod html;
mod injections;
mod javascript;
mod json;
mod languages;
mod python;
mod queries;
mod rust;
mod stability;
mod support;
mod typescript;

#[test]
fn multiline_comments_reach_the_visible_window() {
    let text = "fn main() {\n    /* first\n    second\n    third */\n    let x = 1;\n}\n";
    let spans = highlight(LanguageId::Rust, text, 2, 3);
    assert_eq!(name_at(&spans, 2, 4), Some("comment"));
    assert_eq!(name_at(&spans, 1, 4), Some("comment"));
}

#[test]
fn incremental_parse_updates_later_rows() {
    let runtime = Runtime::new(LanguageId::Rust).expect("runtime");
    let mut document = Document::new(LanguageId::Rust);
    let first = "fn main() {\n    let x = 1;\n}\n";
    let spans = document.update(&request(first, 0, 3, 1), &runtime).1;
    assert_eq!(name_at(&spans, 1, 4), Some("keyword"));
    let second = "/*\nfn main() {\n    let x = 1;\n}\n";
    let spans = document.update(&request(second, 0, 4, 2), &runtime).1;
    assert_eq!(name_at(&spans, 1, 0), Some("comment"));
    assert_eq!(name_at(&spans, 2, 4), Some("comment"));
}

#[test]
fn unicode_columns_use_characters() {
    let text = "let s = \"é\"; // tail\n";
    let spans = highlight(LanguageId::Rust, text, 0, 1);
    let comment = text.find("//").expect("comment");
    let column = text[..comment].chars().count();
    assert_eq!(name_at(&spans, 0, column), Some("comment"));
}

#[test]
fn large_files_use_bounded_windows() {
    let text = "# marker\n".repeat(60_000);
    assert!(text.len() > WINDOW_PARSE_BYTES);
    let runtime = Runtime::new(LanguageId::Python).expect("runtime");
    let mut document = Document::new(LanguageId::Python);
    let key = SyntaxKey {
        document: 1,
        revision: 1,
        top: 30_000,
        end: 30_020,
        language: LanguageId::Python,
    };
    let first = SyntaxRequest {
        key,
        text: Rope::from_str(&text),
    };
    let (window, spans) = document.update(&first, &runtime);
    assert!(document.large);
    assert_eq!(window, 29_000..31_020);
    assert_eq!(document.window_rows, 29_000..31_020);
    assert_eq!(name_at(&spans, 30_000, 0), Some("comment"));

    let second = SyntaxRequest {
        key: SyntaxKey {
            top: 40_000,
            end: 40_020,
            ..key
        },
        text: Rope::from_str(&text),
    };
    let (window, spans) = document.update(&second, &runtime);
    assert_eq!(window, 39_000..41_020);
    assert_eq!(document.window_rows, 39_000..41_020);
    assert_eq!(name_at(&spans, 40_000, 0), Some("comment"));
}

#[test]
fn large_file_edits_refresh_the_window() {
    let text = "# marker\n".repeat(60_000);
    let runtime = Runtime::new(LanguageId::Python).expect("runtime");
    let mut document = Document::new(LanguageId::Python);
    let key = SyntaxKey {
        document: 1,
        revision: 1,
        top: 0,
        end: 20,
        language: LanguageId::Python,
    };
    document.update(
        &SyntaxRequest {
            key,
            text: Rope::from_str(&text),
        },
        &runtime,
    );
    let edited = text.replacen("# marker", "# changed", 1);
    let (_, spans) = document.update(
        &SyntaxRequest {
            key: SyntaxKey { revision: 2, ..key },
            text: Rope::from_str(&edited),
        },
        &runtime,
    );
    assert!(document.large);
    assert_eq!(document.window_rows, 0..1_020);
    assert_eq!(name_at(&spans, 0, 0), Some("comment"));
}

#[test]
fn windowed_edits_keep_new_windows_correct() {
    let text = "# marker\n".repeat(60_000);
    let runtime = Runtime::new(LanguageId::Python).expect("runtime");
    let mut document = Document::new(LanguageId::Python);
    let key = SyntaxKey {
        document: 1,
        revision: 1,
        top: 0,
        end: 20,
        language: LanguageId::Python,
    };
    document.update(
        &SyntaxRequest {
            key,
            text: Rope::from_str(&text),
        },
        &runtime,
    );
    let edited = text.replacen("# marker", "# changed", 1);
    document.update(
        &SyntaxRequest {
            key: SyntaxKey { revision: 2, ..key },
            text: Rope::from_str(&edited),
        },
        &runtime,
    );
    let (_, spans) = document.update(
        &SyntaxRequest {
            key: SyntaxKey {
                revision: 2,
                top: 40_000,
                end: 40_020,
                ..key
            },
            text: Rope::from_str(&edited),
        },
        &runtime,
    );
    assert_eq!(name_at(&spans, 40_000, 0), Some("comment"));
}

#[test]
fn shrinking_a_large_document_restores_a_full_tree() {
    let text = "# marker\n".repeat(60_000);
    let runtime = Runtime::new(LanguageId::Python).expect("runtime");
    let mut document = Document::new(LanguageId::Python);
    let key = SyntaxKey {
        document: 1,
        revision: 1,
        top: 30_000,
        end: 30_020,
        language: LanguageId::Python,
    };
    document.update(
        &SyntaxRequest {
            key,
            text: Rope::from_str(&text),
        },
        &runtime,
    );
    let small = "def main():\n    return True\n";
    let (_, spans) = document.update(
        &SyntaxRequest {
            key: SyntaxKey {
                revision: 2,
                top: 0,
                end: 2,
                ..key
            },
            text: Rope::from_str(small),
        },
        &runtime,
    );
    assert!(!document.large);
    assert_eq!(document.window_rows, 0..document.index.line_count());
    assert_eq!(name_at(&spans, 0, 0), Some("keyword.function"));
}

#[test]
fn long_lines_are_left_plain() {
    let long = format!("{} // tail\n", "a".repeat(MAX_LINE_BYTES + 1));
    let text = format!("{long}fn main() {{}}\n");
    let spans = highlight(LanguageId::Rust, &text, 0, 2);
    assert_eq!(name_at(&spans, 0, 0), None);
    assert_eq!(name_at(&spans, 1, 0), Some("keyword.function"));
}
