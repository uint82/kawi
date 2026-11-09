mod helpers;

use super::super::*;
use super::support::{name_of, request_for};
use helpers::{runtime_for, update};

#[test]
fn python_single_line_incomplete_keeps_parameter() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def two_sum(nums: list[int]) -> list[int]:\n    return nums\n";
    let spans = update(&mut document, &runtime, language, first, 1);
    assert_eq!(
        name_of(&spans, first, 0, "nums"),
        Some("variable.parameter")
    );

    let second = "def two_sum(nums: list[in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(
        name_of(&spans, second, 0, "nums"),
        Some("variable.parameter")
    );
}

#[test]
fn python_incomplete_annotation_with_body_keeps_parameter() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def two_sum(nums: int):\n    return nums\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def two_sum(nums: list[in\n    return nums\n";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(
        name_of(&spans, second, 0, "nums"),
        Some("variable.parameter")
    );
}

#[test]
fn python_new_tokens_are_not_stabilized() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(x: list[in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
    assert_ne!(
        name_of(&spans, second, 0, "list"),
        Some("variable.parameter")
    );
}

#[test]
fn python_changed_parameter_name_is_not_stabilized() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(y: list[in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_ne!(name_of(&spans, second, 0, "y"), Some("variable.parameter"));
}

#[test]
fn extended_parameter_token_is_not_stabilized() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(xx: list[in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_ne!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn prefixed_parameter_token_is_not_stabilized() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(yx: list[in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_ne!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn python_insertion_before_parameter_keeps_capture() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let shifted = "def ff(x: int): pass\n";
    update(&mut document, &runtime, language, shifted, 2);

    let second = "def ff(x: list[in";
    let spans = update(&mut document, &runtime, language, second, 3);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn python_deletion_before_parameter_keeps_capture() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def foo(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let shifted = "def oo(x: int): pass\n";
    update(&mut document, &runtime, language, shifted, 2);

    let second = "def oo(x: list[in";
    let spans = update(&mut document, &runtime, language, second, 3);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn python_newline_before_parameter_keeps_capture() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let shifted = "\ndef f(x: int): pass\n";
    update(&mut document, &runtime, language, shifted, 2);

    let second = "\ndef f(x: list[in";
    let spans = update(&mut document, &runtime, language, second, 3);
    assert_eq!(name_of(&spans, second, 1, "x"), Some("variable.parameter"));
}

#[test]
fn multiple_incomplete_edits_keep_parameter() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(x: list[in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));

    let third = "def f(x: list[int";
    let spans = update(&mut document, &runtime, language, third, 3);
    assert_eq!(name_of(&spans, third, 0, "x"), Some("variable.parameter"));
}

#[test]
fn valid_parse_refreshes_the_stable_snapshot() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(x: list[in";
    update(&mut document, &runtime, language, second, 2);

    let third = "def f(x: str): pass\n";
    update(&mut document, &runtime, language, third, 3);

    let fourth = "def f(x: list[in";
    let spans = update(&mut document, &runtime, language, fourth, 4);
    assert_eq!(name_of(&spans, fourth, 0, "x"), Some("variable.parameter"));
}

#[test]
fn current_specific_capture_is_not_downgraded() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "foo(x)\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def foo(x: int\n    return x\n";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn unrelated_rows_use_current_highlights() {
    let language = LanguageId::Rust;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "fn main() {}\nfn other() {}\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "struct main {}\nfn other(x: Vec<in\n";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "struct"), Some("keyword"));
}

#[test]
fn rust_incomplete_annotation_keeps_parameter() {
    let language = LanguageId::Rust;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "fn add(x: i32) -> i32 { x }\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "fn add(x: Vec<in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn typescript_incomplete_annotation_keeps_parameter() {
    let language = LanguageId::TypeScript;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "function add(x: number) { return x; }\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "function add(x: Array<in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn go_incomplete_annotation_keeps_parameter() {
    let language = LanguageId::Go;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "package main\nfunc add(x int) int { return x }\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "package main\nfunc add(x []in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 1, "x"), Some("variable.parameter"));
}

#[test]
fn unicode_parameter_keeps_capture() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(é: int): pass\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(é: list[in";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "é"), Some("variable.parameter"));
}

#[test]
fn crlf_parameter_keeps_capture() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let first = "def f(x: int): pass\r\n";
    update(&mut document, &runtime, language, first, 1);

    let second = "def f(x: list[in\r\n";
    let spans = update(&mut document, &runtime, language, second, 2);
    assert_eq!(name_of(&spans, second, 0, "x"), Some("variable.parameter"));
}

#[test]
fn first_incomplete_parse_has_no_stable_snapshot() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let text = "def f(x: list[in";
    let spans = update(&mut document, &runtime, language, text, 1);
    assert_ne!(name_of(&spans, text, 0, "x"), Some("variable.parameter"));
}

#[test]
fn large_file_window_keeps_parameter() {
    let language = LanguageId::Python;
    let runtime = runtime_for(language);
    let mut document = Document::new(language);
    let filler = "# marker\n".repeat(60_000);
    let first = format!("def f(x: int): pass\n{filler}");
    let request = request_for(language, &first, 0, 2, 1);
    document.update(&request, &runtime);

    let second = format!("def f(x: list[in\n{filler}");
    let request = request_for(language, &second, 0, 2, 2);
    let (_, spans) = document.update(&request, &runtime);
    assert_eq!(name_of(&spans, &second, 0, "x"), Some("variable.parameter"));
}
