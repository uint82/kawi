use super::*;
use support::{highlight, name_of};

#[test]
fn highlights_java_keywords_operators_and_literals() {
    let text = r#"class Syntax {
    void run(int value) {
        if (value > 0 && value <= 10) { value++; }
        for (int index = 0; index < value; index++) { continue; }
        while (value-- > 0) { break; }
        switch (value) { case 1: break; default: value += 1; }
        try { throw new Error(); } catch (RuntimeException error) { } finally { }
        assert value != 0 : "zero";
        String text = """
            hello
            """;
        String name = "value";
        String template = STR."value: \{value}";
        int hex = 0xFF;
        long bits = 1L;
        double ratio = 1.5;
        char letter = 'x';
        boolean enabled = true;
        Object nothing = null;
        int shift = value >>> 1;
        if (value instanceof Integer number) { }
    }
}
"#;
    let spans = highlight(LanguageId::Java, text, 0, 20);
    assert_eq!(name_of(&spans, text, 2, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 2, ">"), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "&&"), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "<="), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "++"), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 3, "int"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 3, "index"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "continue"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 4, "while"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 4, "--"), Some("operator"));
    assert_eq!(name_of(&spans, text, 4, "break"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 5, "switch"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 5, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 5, "default"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 5, "+="), Some("operator"));
    assert_eq!(name_of(&spans, text, 6, "try"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 6, "throw"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 6, "new"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 6, "Error"), Some("type"));
    assert_eq!(name_of(&spans, text, 6, "catch"), Some("keyword.exception"));
    assert_eq!(
        name_of(&spans, text, 6, "finally"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 7, "assert"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 7, "!="), Some("operator"));
    assert_eq!(name_of(&spans, text, 8, "\"\"\""), Some("string"));
    assert_eq!(name_of(&spans, text, 11, "\"value\""), Some("string"));
    assert_eq!(
        name_of(&spans, text, 12, "\\{"),
        Some("punctuation.special")
    );
    assert_eq!(name_of(&spans, text, 13, "0xFF"), Some("number"));
    assert_eq!(name_of(&spans, text, 14, "1L"), Some("number"));
    assert_eq!(name_of(&spans, text, 15, "1.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 16, "'x'"), Some("character"));
    assert_eq!(name_of(&spans, text, 17, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 18, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 19, ">>>"), Some("operator"));
    assert_eq!(name_of(&spans, text, 20, "instanceof"), Some("keyword"));
}

#[test]
fn highlights_java_annotations_labels_and_comments() {
    let text = r#"/**
 * Docs.
 */
@java.lang.Deprecated
@SuppressWarnings("unchecked")
class Marked {
    void run() {
        outer:
        for (int i = 0; i < 1; i++) {
            break outer;
        }
        // line comment
        /* block comment */
    }
}
"#;
    let spans = highlight(LanguageId::Java, text, 0, 13);
    assert_eq!(
        name_of(&spans, text, 0, "/**"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 3, "Deprecated"), Some("attribute"));
    assert_eq!(
        name_of(&spans, text, 4, "SuppressWarnings"),
        Some("attribute")
    );
    assert_eq!(name_of(&spans, text, 7, "outer"), Some("label"));
    assert_eq!(name_of(&spans, text, 9, "outer"), Some("label"));
    assert_eq!(name_of(&spans, text, 11, "//"), Some("comment"));
    assert_eq!(name_of(&spans, text, 12, "/*"), Some("comment"));
}
