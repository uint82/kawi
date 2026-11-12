use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_rust_declarations_and_members() {
    let text = "#[derive(Debug, Clone)]\npub struct Point<'a, T: Copy> {\n    pub x: &'a T,\n    field: u8,\n}\nenum State { Ready, Waiting(u8) }\npub trait Shape {\n    fn area(&self) -> f64;\n}\nimpl<'a> Shape for Point<'a, u8> {\n    fn area(&self) -> f64 { 0.0 }\n}\n";
    let spans = highlight(LanguageId::Rust, text, 0, 10);
    assert_eq!(name_of(&spans, text, 0, "derive"), Some("function.macro"));
    assert_eq!(name_of(&spans, text, 0, "Debug"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "pub"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 1, "struct"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 1, "Point"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "'"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 1, "a"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 1, "T"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 1, "Copy"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "x"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 3, "field"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 3, "u8"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 5, "enum"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 5, "State"), Some("type"));
    assert_eq!(name_of(&spans, text, 5, "Ready"), Some("constant"));
    assert_eq!(name_of(&spans, text, 5, "Waiting"), Some("constant"));
    assert_eq!(name_of(&spans, text, 6, "trait"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 6, "Shape"), Some("type"));
    assert_eq!(name_of(&spans, text, 7, "fn"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 7, "area"), Some("function"));
    assert_eq!(name_of(&spans, text, 7, "self"), Some("variable.builtin"));
    assert_eq!(name_of(&spans, text, 7, "f64"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 9, "impl"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 9, "for"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 10, "0.0"), Some("number.float"));
}

#[test]
fn highlights_rust_keywords_and_calls() {
    let text = "use std::fmt;\nmod inner;\nfn convert(value: u8) -> u16 { value as u16 }\nfn controls(mut value: u8) -> u8 {\n    if value > 0 { while value < 3 { value += 1; } } else { return 0; }\n    for item in 0..=3 { match item { 1 => continue, _ => break } }\n}\nfn accept(value: &dyn Shape) { let ref copy = value; let _ = move || copy; }\nfn items() { self.values.push(1); }\n";
    let spans = highlight(LanguageId::Rust, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "use"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 0, "std"), Some("module"));
    assert_eq!(name_of(&spans, text, 1, "mod"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "inner"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "as"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "u16"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 3, "mut"), Some("keyword.modifier"));
    assert_eq!(
        name_of(&spans, text, 3, "value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 4, "if"), Some("keyword.conditional"));
    assert_eq!(
        name_of(&spans, text, 4, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 4, "while"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 4, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 5, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 5, "in"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 5, "match"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 5, "continue"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 5, "break"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 7, "dyn"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 7, "ref"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 7, "move"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 8, "push"), Some("function.call"));
}

#[test]
fn highlights_rust_literals_and_macros() {
    let text = "fn literals() {\n    let ok = true;\n    let no = false;\n    let count = 42;\n    let ratio = 0.0;\n    let letter = 'x';\n    let text = \"hi\\n\";\n    let literal = r#\"raw\"#;\n    let pattern = regex::Regex::new(r\"^[a-z]+$\").unwrap();\n    println!(\"hi\");\n    dbg!(count);\n    assert_eq!(count, 42);\n    panic!(\"no\");\n}\n";
    let spans = highlight(LanguageId::Rust, text, 0, 13);
    assert_eq!(name_of(&spans, text, 1, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 2, "false"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 3, "42"), Some("number"));
    assert_eq!(name_of(&spans, text, 4, "0.0"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 5, "x"), Some("character"));
    assert_eq!(name_of(&spans, text, 6, "hi"), Some("string"));
    assert_eq!(name_of(&spans, text, 6, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 7, "raw"), Some("string"));
    assert_eq!(name_of(&spans, text, 8, "^[a-z]+$"), Some("string.regexp"));
    assert_eq!(name_of(&spans, text, 9, "println"), Some("function.macro"));
    assert_eq!(name_of(&spans, text, 10, "dbg"), Some("keyword.debug"));
    assert_eq!(
        name_of(&spans, text, 11, "assert_eq"),
        Some("keyword.exception")
    );
    assert_eq!(
        name_of(&spans, text, 12, "panic"),
        Some("keyword.exception")
    );
}

#[test]
fn highlights_rust_lifetimes_labels_and_attributes() {
    let text = "#!/usr/bin/env rustx\n//! Crate docs\n/// Item docs\n#[cfg(test)]\nfn borrow<'a, 'static>(value: &'a str, other: &'static str) {\n    let closure = |arg: u8| -> u8 { arg + 1 };\n    'outer: loop { break 'outer; }\n}\n";
    let spans = highlight(LanguageId::Rust, text, 0, 8);
    assert_eq!(name_at(&spans, 0, 0), Some("keyword.directive"));
    assert_eq!(name_at(&spans, 1, 0), Some("comment.documentation"));
    assert_eq!(name_at(&spans, 2, 0), Some("comment.documentation"));
    assert_eq!(name_of(&spans, text, 3, "#"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 3, "cfg"), Some("function.macro"));
    assert_eq!(name_of(&spans, text, 4, "'"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 4, "a"), Some("attribute"));
    assert_eq!(
        name_of(&spans, text, 4, "static"),
        Some("attribute.builtin")
    );
    assert_eq!(name_of(&spans, text, 4, "str"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 5, "|"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 5, "arg"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 5, "u8"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 6, "outer"), Some("label"));
    assert_eq!(name_of(&spans, text, 6, "loop"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 6, "break"), Some("keyword.repeat"));
}

#[test]
fn highlights_rust_builtin_variants_and_wildcards() {
    let text = "fn values() -> Option<u8> {\n    let value = Some(1);\n    match value { Some(inner) => inner, None => 0, _ => 1 }\n}\n";
    let spans = highlight(LanguageId::Rust, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "Option"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "Some"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 2, "Some"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 2, "inner"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "None"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 2, "_"), Some("character.special"));
}
