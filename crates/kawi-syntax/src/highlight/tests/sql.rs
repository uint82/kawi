use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_postgresql_query_structure() {
    let text = "WITH active_users AS (\n    SELECT id, email::text AS mailbox\n    FROM public.users\n    WHERE active = true AND score >= 1.5\n)\nSELECT q.id, COUNT(*) AS total\nFROM active_users q\nWHERE q.email ILIKE 'A%'\nGROUP BY u.id;\n";
    let spans = highlight(LanguageId::Sql, text, 0, 9);
    assert_eq!(name_of(&spans, text, 0, "WITH"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "active_users"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "SELECT"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "id"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 1, "text"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "mailbox"), Some("variable"));
    assert_eq!(name_of(&spans, text, 2, "users"), Some("type"));
    assert_eq!(name_of(&spans, text, 2, "public"), None);
    assert_eq!(name_of(&spans, text, 3, "active"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 3, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 3, "1.5"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 5, "COUNT"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 5, "total"), Some("variable"));
    assert_eq!(name_of(&spans, text, 6, "q"), Some("variable"));
    assert_eq!(name_of(&spans, text, 7, "ILIKE"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 7, "'A%'"), Some("string"));
    assert_eq!(name_of(&spans, text, 8, "."), Some("punctuation.delimiter"));
}

#[test]
fn highlights_postgresql_literals_parameters_and_comments() {
    let text = "INSERT INTO audit_log (user_id, action, payload)\nVALUES ($1, 'login', $$json$$, 42, 1.5e3, NULL, TRUE);\n-- note\n/* block */\n";
    let spans = highlight(LanguageId::Sql, text, 0, 4);
    assert_eq!(name_of(&spans, text, 0, "audit_log"), Some("type"));
    assert_eq!(name_of(&spans, text, 0, "user_id"), Some("variable.member"));
    assert_eq!(name_of(&spans, text, 1, "$1"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 1, "'login'"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "$$json$$"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "42"), Some("number"));
    assert_eq!(name_of(&spans, text, 1, "1.5e3"), Some("number.float"));
    assert_eq!(name_of(&spans, text, 1, "NULL"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "TRUE"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 2, "-- note"), Some("comment"));
    assert_eq!(name_of(&spans, text, 3, "/* block */"), Some("comment"));
}

#[test]
fn highlights_postgresql_functions_and_windows() {
    let text = "SELECT COUNT(*) FILTER (WHERE paid) OVER (PARTITION BY team_id ORDER BY id) AS paid_orders\nFROM orders;\n";
    let spans = highlight(LanguageId::Sql, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "COUNT"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 0, "FILTER"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "OVER"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "PARTITION"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "paid_orders"), Some("variable"));
    assert_eq!(name_of(&spans, text, 0, "ORDER"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "BY"), Some("keyword.operator"));
}

#[test]
fn highlights_postgresql_ddl_types_and_constraints() {
    let text = "CREATE TABLE IF NOT EXISTS demo (\n    id BIGSERIAL PRIMARY KEY,\n    name TEXT NOT NULL UNIQUE,\n    total NUMERIC(12, 2) CHECK (total >= 0),\n    payload JSONB DEFAULT '{}'::jsonb,\n    owner_id UUID REFERENCES users(id) ON DELETE CASCADE\n);\n";
    let spans = highlight(LanguageId::Sql, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "CREATE"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "demo"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "BIGSERIAL"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "PRIMARY"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 2, "TEXT"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "NOT"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 2, "NULL"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 2, "UNIQUE"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 3, "NUMERIC"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 3, "CHECK"), Some("keyword.modifier"));
    assert_eq!(name_of(&spans, text, 3, ">="), Some("operator"));
    assert_eq!(name_of(&spans, text, 4, "JSONB"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 4, "DEFAULT"), Some("attribute"));
    assert_eq!(
        name_of(&spans, text, 5, "owner_id"),
        Some("variable.member")
    );
    assert_eq!(name_of(&spans, text, 5, "UUID"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 5, "REFERENCES"), Some("keyword"));
    assert_eq!(
        name_of(&spans, text, 5, "CASCADE"),
        Some("keyword.modifier")
    );
}
