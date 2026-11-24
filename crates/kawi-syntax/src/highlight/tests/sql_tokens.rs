use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_sql_keywords_and_operators() {
    let text = "SELECT name FROM users WHERE id IN (1, 2) AND active IS NOT NULL ORDER BY name DESC LIMIT 10;\n";
    let spans = highlight(LanguageId::Sql, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "SELECT"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "FROM"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "WHERE"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "IN"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 0, "AND"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 0, "IS"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "NOT"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 0, "ORDER"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "BY"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 0, "DESC"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 0, "LIMIT"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "("), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 0, ","), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 0, ";"), Some("punctuation.delimiter"));
}

#[test]
fn highlights_sql_conditionals_and_modifiers() {
    let text = "SELECT CASE WHEN paid THEN 'yes' ELSE 'no' END AS status\nFROM orders\nWHERE amount > 0 AND status = 'paid';\n";
    let spans = highlight(LanguageId::Sql, text, 0, 3);
    assert_eq!(
        name_of(&spans, text, 0, "CASE"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 0, "WHEN"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 0, "THEN"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 0, "ELSE"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 0, "END"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "'yes'"), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "status"), Some("variable"));
    assert_eq!(name_of(&spans, text, 1, "orders"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, ">"), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "0"), Some("number"));
}

#[test]
fn highlights_sql_array_and_cast_keywords() {
    let text = "SELECT ARRAY[1, 2], value::INTEGER, (payload->>'name')::TEXT\nFROM items;\n";
    let spans = highlight(LanguageId::Sql, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "ARRAY"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 0, "INTEGER"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "TEXT"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "name"), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "1"), Some("number"));
    assert_eq!(name_of(&spans, text, 0, "->>"), Some("operator"));
}

#[test]
fn highlights_sql_cast_and_extract_calls() {
    let text = "SELECT CAST(score AS INTEGER), EXTRACT(YEAR FROM created_at)\nFROM events;\n";
    let spans = highlight(LanguageId::Sql, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "CAST"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 0, "INTEGER"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 0, "EXTRACT"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 0, "YEAR"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 0, "FROM"), Some("keyword"));
}
