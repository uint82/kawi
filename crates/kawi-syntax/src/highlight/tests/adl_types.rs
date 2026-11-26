use super::support::{highlight, name_at, name_of};
use super::*;

fn name_of_first(spans: &[SyntaxSpan], text: &str, needle: &str) -> Option<&'static str> {
    for (row, line) in text.lines().enumerate() {
        if let Some(column) = line.find(needle) {
            return name_at(spans, row, column);
        }
    }
    None
}

#[test]
fn highlights_adl_builtin_types() {
    let text = "module demo {\nstruct Primitives\n{\n  Void v;\n  Bool b;\n  Int8 i8;\n  Int16 i16;\n  Int32 i32;\n  Int64 i64;\n  Word8 w8;\n  Word16 w16;\n  Word32 w32;\n  Word64 w64;\n  Float f;\n  Double d;\n  Json j;\n  Bytes bytes;\n  String s;\n  Vector<String> vec;\n  StringMap<String> map;\n  Nullable<String> opt;\n  TypeToken<String> token;\n};\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 23);
    for name in [
        "Void",
        "Bool",
        "Int8",
        "Int16",
        "Int32",
        "Int64",
        "Word8",
        "Word16",
        "Word32",
        "Word64",
        "Float",
        "Double",
        "Json",
        "Bytes",
        "String",
        "Vector",
        "StringMap",
        "Nullable",
        "TypeToken",
    ] {
        assert_eq!(
            name_of_first(&spans, text, name),
            Some("type.builtin"),
            "{name}"
        );
    }
}

#[test]
fn highlights_adl_type_parameters() {
    let text = "module demo {\nstruct Pair<T1, T2>\n{\n  T1 first;\n  T2 second;\n};\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 7);
    assert_eq!(name_of(&spans, text, 1, "T1"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 1, "T2"), Some("type.parameter"));
    assert_eq!(name_of(&spans, text, 3, "T1"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "T2"), Some("type"));
}

#[test]
fn highlights_adl_user_types_and_scoped_names() {
    let text = "module demo {\ntype UserVec = Vector<sys.types.Pair<Int32, String>>;\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 3);
    assert_eq!(name_of(&spans, text, 1, "UserVec"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 1, "Vector"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "sys.types.Pair"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "Int32"), Some("type.builtin"));
    assert_eq!(name_of(&spans, text, 1, "String"), Some("type.builtin"));
}
