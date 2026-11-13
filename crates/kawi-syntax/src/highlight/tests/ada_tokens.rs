use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_ada_literals_and_booleans() {
    let text = r#"Name : String := "hello"; -- Comment.
Char : Character := 'A';
Count : Integer := 1_000;
Hex : Integer := 16#FF#;
Ratio : Float := 1.0E+3;
Flag : Boolean := True;
Other : Boolean := FALSE;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "-- Comment."), Some("comment"));
    assert_eq!(name_of(&spans, text, 0, "\"hello\""), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "'A'"), Some("character"));
    assert_eq!(name_of(&spans, text, 2, "1_000"), Some("number"));
    assert_eq!(name_of(&spans, text, 3, "16#FF#"), Some("number"));
    assert_eq!(name_of(&spans, text, 4, "1.0E+3"), Some("number"));
    assert_eq!(name_of(&spans, text, 5, "True"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 6, "FALSE"), Some("boolean"));
    assert_eq!(name_of(&spans, text, 0, ":"), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 0, ";"), Some("punctuation.delimiter"));
}

#[test]
fn highlights_ada_keywords_and_operators() {
    let text = r#"if Ready and then not Done or else Fallback then
   return;
end if;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "if"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 0, "and"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 0, "not"), Some("keyword.operator"));
    assert_eq!(name_of(&spans, text, 0, "or"), Some("keyword.operator"));
    assert_eq!(
        name_of(&spans, text, 0, "else"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 1, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 2, "end"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 2, "if"), Some("keyword.conditional"));
}

#[test]
fn highlights_ada_symbolic_operators() {
    let text = r#"Value := A + B * C ** 2;
Span := 1 .. 10;
Set := A | B;
Check := X => Y;
Compare := X /= Y and X <= Y;
Magnitude := abs (Value);
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 6);
    assert_eq!(name_of(&spans, text, 0, ":="), Some("operator"));
    assert_eq!(name_of(&spans, text, 0, "+"), Some("operator"));
    assert_eq!(name_of(&spans, text, 0, "* C"), Some("operator"));
    assert_eq!(name_of(&spans, text, 0, "**"), Some("operator"));
    assert_eq!(name_of(&spans, text, 1, ".."), Some("operator"));
    assert_eq!(name_of(&spans, text, 2, "|"), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, "=>"), Some("operator"));
    assert_eq!(name_of(&spans, text, 4, "/="), Some("operator"));
    assert_eq!(name_of(&spans, text, 4, "<="), Some("operator"));
    assert_eq!(name_of(&spans, text, 5, "abs"), Some("keyword.operator"));
}

#[test]
fn highlights_ada_imports() {
    let text = r#"with Ada.Text_IO;
use Ada.Text_IO;
use type Ada.Text_IO.File_Type;
private with Ada.Finalization;
limited with Ada.Streams;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "with"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 0, "Ada"), Some("module"));
    assert_eq!(name_of(&spans, text, 0, "Text_IO"), Some("module"));
    assert_eq!(name_of(&spans, text, 1, "use"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 1, "Ada"), Some("module"));
    assert_eq!(name_of(&spans, text, 2, "type"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 2, "File_Type"), Some("module"));
    assert_eq!(name_of(&spans, text, 3, "private"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 3, "with"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 4, "limited"), Some("keyword.import"));
    assert_eq!(name_of(&spans, text, 4, "Streams"), Some("module"));
}
