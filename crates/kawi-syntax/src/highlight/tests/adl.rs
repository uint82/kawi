use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_adl_modules_and_imports() {
    let text = "module demo.sample\n{\nimport sys.types.*;\nimport demo.types.Foo;\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "module"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "demo.sample"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "import"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 2, "sys.types"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, ".*"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 3, "demo.types.Foo"), Some("module"));
}

#[test]
fn highlights_adl_declarations_and_fields() {
    let text = "module demo {\nstruct Person#2\n{\n  String firstName;\n  Gender gender;\n};\nunion Gender\n{\n  Void male;\n};\ntype UserVec<T> = Vector<T>;\nnewtype UserId = String;\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 13);
    assert_eq!(name_of(&spans, text, 1, "struct"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "Person"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 1, "#2"), Some("number"));
    assert_eq!(name_of(&spans, text, 3, "String"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 3, "firstName"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "Gender"), Some("type"));
    assert_eq!(name_of(&spans, text, 6, "Gender"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 8, "Void"), Some("type.builtin"));
    assert_eq!(
        name_of(&spans, text, 10, "UserVec"),
        Some("type.definition")
    );
    assert_eq!(name_of(&spans, text, 10, "T"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 10, "Vector"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 11, "UserId"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 11, "String"), Some("type.builtin"));
}

#[test]
fn highlights_adl_annotations_and_defaults() {
    let text = "module demo {\n@DbTable {\n  \"tableName\": \"demo.users\",\n  \"primaryKey\": [\"email\"],\n  \"enabled\": true\n}\nstruct User\n{\n  @DbField \"email\"\n  String email;\n  Int8 rank = 1;\n};\nannotation Address DbTable {\n  \"tableName\": \"demo.address\"\n};\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 16);
    assert_eq!(name_of(&spans, text, 1, "@"), Some("punctuation.special"));
    assert_eq!(name_of(&spans, text, 1, "DbTable"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 2, "\"tableName\""), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "\"demo.users\""), Some("string"));
    assert_eq!(name_of(&spans, text, 4, "true"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 8, "DbField"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 9, "String"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 9, "email"), Some("property"));
    assert_eq!(name_of(&spans, text, 10, "="), Some("operator"));
    assert_eq!(name_of(&spans, text, 10, "1"), Some("number"));
    assert_eq!(name_of(&spans, text, 12, "annotation"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 12, "Address"), Some("type"));
    assert_eq!(name_of(&spans, text, 12, "DbTable"), Some("attribute"));
}

#[test]
fn highlights_adl_comments_and_docstrings() {
    let text = "/// Module docs.\nmodule demo {\n  // note\n  struct User {}\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 5);
    assert_eq!(
        name_of(&spans, text, 0, "/// Module docs."),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 2, "// note"), Some("comment"));
    assert_eq!(name_of(&spans, text, 3, "struct"), Some("keyword"));
}

#[test]
fn adl_builtin_types_do_not_capture_declarations_or_modules() {
    let text = "module String {\n  struct String {\n    Int8 value;\n  };\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "String"), Some("module"));
    assert_eq!(name_of(&spans, text, 1, "String"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 2, "Int8"), Some("type.builtin"));
}

#[test]
fn adl_builtin_type_matching_is_exact() {
    let text = "module demo {\nstruct Box {\n  Stringy value;\n  Strings values;\n  sys.types.Pair item;\n};\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 7);
    assert_eq!(name_of(&spans, text, 2, "Stringy"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "Strings"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "sys.types.Pair"), Some("type"));
}

#[test]
fn adl_highlights_incomplete_source() {
    let text = "module demo {\n  struct User {\n    String name;\n  };\n";
    let spans = highlight(LanguageId::Adl, text, 0, 4);
    assert_eq!(name_of(&spans, text, 0, "module"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "struct"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "User"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 2, "String"), Some("type.builtin"));
}

#[test]
fn adl_comments_only_source_stays_plain() {
    let text = "// no declarations here\n";
    let spans = highlight(LanguageId::Adl, text, 0, 1);
    assert_eq!(
        name_of(&spans, text, 0, "// no declarations here"),
        Some("comment")
    );
}
