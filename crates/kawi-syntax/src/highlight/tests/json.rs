use super::support::{highlight, name_at, name_of};
use super::*;

#[test]
fn highlights_json_pairs_and_scalars() {
    let text = "{\"name\": \"kawi\", \"count\": 2, \"ratio\": -1.5e+2, \"enabled\": true, \"missing\": null}\n";
    let spans = highlight(LanguageId::Json, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "name"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "kawi"), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "count"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "ratio"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "-1.5e+2"), Some("number"));
    assert_eq!(name_of(&spans, text, 0, "enabled"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "true"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 0, "missing"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "null"), Some("constant.builtin"));
    let colon = text.find(':').expect(":");
    assert_eq!(name_at(&spans, 0, colon), Some("punctuation.delimiter"));
    let comma = text.find(',').expect(",");
    assert_eq!(name_at(&spans, 0, comma), Some("punctuation.delimiter"));
}

#[test]
fn highlights_json_nested_containers() {
    let text = "{\n  \"items\": [\n    {\"id\": 1, \"tags\": [\"a\", \"b\"]},\n    {\"id\": 2, \"tags\": []}\n  ]\n}\n";
    let spans = highlight(LanguageId::Json, text, 0, 6);
    assert_eq!(name_of(&spans, text, 1, "items"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "id"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "1"), Some("number"));
    assert_eq!(name_of(&spans, text, 2, "tags"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "\"a\""), Some("string"));
    assert_eq!(name_of(&spans, text, 2, "\"b\""), Some("string"));
    assert_eq!(name_of(&spans, text, 3, "id"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "2"), Some("number"));
    let line = text.lines().nth(1).expect("row");
    assert_eq!(
        name_at(&spans, 1, line.find('[').expect("[")),
        Some("punctuation.bracket")
    );
    assert_eq!(
        name_at(&spans, 1, line.find(':').expect(":")),
        Some("punctuation.delimiter")
    );
    let line = text.lines().nth(2).expect("row");
    assert_eq!(
        name_at(&spans, 2, line.find(',').expect(",")),
        Some("punctuation.delimiter")
    );
}

#[test]
fn highlights_json_comments_and_escapes() {
    let text = r#"{
  // note
  "path": "a\nb",
  "quote": "\""
}
"#;
    let spans = highlight(LanguageId::Json, text, 0, 5);
    assert_eq!(name_of(&spans, text, 1, "note"), Some("comment"));
    assert_eq!(name_of(&spans, text, 2, "path"), Some("property"));
    let line = text.lines().nth(2).expect("row");
    assert_eq!(
        name_at(&spans, 2, line.find("\\n").expect("\\n")),
        Some("string.escape")
    );
    assert_eq!(name_of(&spans, text, 3, "quote"), Some("property"));
    let line = text.lines().nth(3).expect("row");
    assert_eq!(
        name_at(&spans, 3, line.find("\\\"").expect("\\\"")),
        Some("string.escape")
    );
}

#[test]
fn highlights_json_top_level_scalars() {
    for (text, capture) in [
        ("\"hello\"\n", "string"),
        ("42\n", "number"),
        ("false\n", "boolean"),
        ("null\n", "constant.builtin"),
    ] {
        let spans = highlight(LanguageId::Json, text, 0, 1);
        assert_eq!(name_at(&spans, 0, 0), Some(capture), "{text}");
    }
}

#[test]
fn highlights_json_empty_containers() {
    let text = "{\"object\": {}, \"array\": []}\n";
    let spans = highlight(LanguageId::Json, text, 0, 1);
    assert_eq!(name_of(&spans, text, 0, "object"), Some("property"));
    assert_eq!(name_of(&spans, text, 0, "array"), Some("property"));
    let object = text.find("{}").expect("{}");
    assert_eq!(name_at(&spans, 0, object), Some("punctuation.bracket"));
    let array = text.find("[]").expect("[]");
    assert_eq!(name_at(&spans, 0, array), Some("punctuation.bracket"));
}
