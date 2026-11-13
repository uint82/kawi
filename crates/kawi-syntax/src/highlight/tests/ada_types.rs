use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_ada_types_and_subtypes() {
    let text = r#"type Color is (Red, Green, Blue);
type Point is record
   X : Integer;
   Y : Integer;
end record;
subtype Small is Integer range 1 .. 10;
type Ptr is access all Integer;
type Matrix is array (1 .. 3) of Float;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 7);
    assert_eq!(name_of(&spans, text, 0, "Color"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 0, "is"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 0, "Red"), Some("constant"));
    assert_eq!(name_of(&spans, text, 0, "Green"), Some("constant"));
    assert_eq!(name_of(&spans, text, 0, "Blue"), Some("constant"));
    assert_eq!(name_of(&spans, text, 1, "Point"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 1, "record"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 2, "X"), Some("property"));
    assert_eq!(name_of(&spans, text, 3, "Y"), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "end"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 5, "Small"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 5, "subtype"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 5, "range"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 6, "Ptr"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 6, "access"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 6, "all"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 7, "Matrix"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 7, "array"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 7, "of"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 7, "Float"), Some("type"));
}

#[test]
fn highlights_ada_tasks_protected_entries_and_accepts() {
    let text = r#"task Worker;
task body Worker is
begin
   accept Start (Count : Integer) do
      null;
   end Start;
end Worker;

task type Job is
   entry Go (Count : Integer);
end Job;

protected type Buffer is
   procedure Put (Item : Integer);
end Buffer;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 14);
    assert_eq!(name_of(&spans, text, 0, "Worker"), Some("variable"));
    assert_eq!(name_of(&spans, text, 1, "Worker"), Some("type"));
    assert_eq!(name_of(&spans, text, 3, "Start"), Some("function.call"));
    assert_eq!(
        name_of(&spans, text, 3, "Count"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 8, "Job"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 8, "task"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 9, "Go"), Some("function"));
    assert_eq!(name_of(&spans, text, 12, "Buffer"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 12, "protected"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 13, "Put"), Some("function"));
}

#[test]
fn highlights_ada_generics_and_formal_parameters() {
    let text = r#"generic
   type Element is private;
   Size : Integer := 10;
   with procedure Visit (Item : Integer);
package Containers is
   package Vectors is new Ada.Containers.Vectors (Element);
end Containers;

function To_String is new Conversion (Integer);
procedure Run is new Runner (Integer);
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 9);
    assert_eq!(name_of(&spans, text, 1, "Element"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 1, "private"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 2, "Size"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 3, "with"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 3, "Visit"), Some("function"));
    assert_eq!(name_of(&spans, text, 3, "Item"), Some("variable.parameter"));
    assert_eq!(name_of(&spans, text, 4, "Containers"), Some("module"));
    assert_eq!(name_of(&spans, text, 5, "Vectors"), Some("module"));
    assert_eq!(
        name_of(&spans, text, 5, "Ada.Containers.Vectors"),
        Some("module")
    );
    assert_eq!(name_of(&spans, text, 8, "To_String"), Some("function"));
    assert_eq!(name_of(&spans, text, 8, "Conversion"), Some("function"));
    assert_eq!(name_of(&spans, text, 9, "Run"), Some("function"));
    assert_eq!(name_of(&spans, text, 9, "Runner"), Some("function"));
}

#[test]
fn highlights_ada_aspects_and_expression_functions() {
    let text = r#"procedure Run (Value : Integer)
  with Pre => Value > 0,
       Post => Value > 0;

function Image (Value : Integer) return String is
  (Value'Image);
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 5);
    assert_eq!(name_of(&spans, text, 1, "with"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "Pre"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 2, "Post"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 4, "is"), Some("keyword.function"));
    assert_eq!(name_of(&spans, text, 5, "Image"), Some("attribute"));
}
