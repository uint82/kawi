use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_ada_definitions_and_parameters() {
    let text = r#"package Demo is
   procedure Run (Value : Integer);
   function Image (Value : Integer) return String;
end Demo;

package body Demo is
   procedure Run (Value : Integer) is
   begin
      null;
   end Run;
end Demo;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 10);
    assert_eq!(name_of(&spans, text, 0, "package"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 0, "Demo"), Some("module"));
    assert_eq!(
        name_of(&spans, text, 1, "procedure"),
        Some("keyword.function")
    );
    assert_eq!(name_of(&spans, text, 1, "Run"), Some("function"));
    assert_eq!(
        name_of(&spans, text, 1, "Value"),
        Some("variable.parameter")
    );
    assert_eq!(name_of(&spans, text, 1, "Integer"), Some("type"));
    assert_eq!(
        name_of(&spans, text, 2, "function"),
        Some("keyword.function")
    );
    assert_eq!(name_of(&spans, text, 2, "Image"), Some("function"));
    assert_eq!(name_of(&spans, text, 2, "return"), Some("keyword.return"));
    assert_eq!(name_of(&spans, text, 2, "String"), Some("type"));
    assert_eq!(name_of(&spans, text, 5, "body"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 5, "Demo"), Some("module"));
    assert_eq!(name_of(&spans, text, 6, "Run"), Some("function"));
    assert_eq!(name_of(&spans, text, 6, "is"), Some("keyword.function"));
}

#[test]
fn highlights_ada_calls_and_attributes() {
    let text = r#"procedure Run is
begin
   Show (1);
   Object.Show (2);
   Result := Integer'Image (3);
   Result := Object.Image (4);
   Result := Integer'Range;
end Run;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 8);
    assert_eq!(name_of(&spans, text, 2, "Show"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 3, "Show"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 4, "Integer"), Some("type"));
    assert_eq!(name_of(&spans, text, 4, "Image"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 4, "'"), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 5, "Image"), Some("function.call"));
    assert_eq!(name_of(&spans, text, 6, "Range"), Some("attribute"));
}

#[test]
fn highlights_ada_objects_constants_and_components() {
    let text = r#"package Demo is
   Limit : constant Integer := 10;
   Count : Integer := 0;
   Renamed : Integer renames Count;
   Number : constant := 42;
   type Point is record
      X : Integer := 0;
   end record;
end Demo;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 8);
    assert_eq!(name_of(&spans, text, 1, "Limit"), Some("constant"));
    assert_eq!(
        name_of(&spans, text, 1, "constant"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 2, "Count"), Some("variable"));
    assert_eq!(name_of(&spans, text, 3, "Renamed"), Some("variable"));
    assert_eq!(
        name_of(&spans, text, 3, "renames"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 4, "Number"), Some("constant"));
    assert_eq!(name_of(&spans, text, 6, "X"), Some("property"));
}

#[test]
fn highlights_ada_exceptions_labels_and_documentation() {
    let text = r#"-- Package docs.
package Demo is
   -- Error docs.
   Error : exception;
   Failure : exception renames Error;
end Demo;

package body Demo is
   procedure Run is
   begin
      Outer : loop
         exit Outer;
      end loop;
      raise Error with "bad";
   exception
      when Choice : Error =>
         null;
   end Run;
end Demo;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 18);
    assert_eq!(
        name_of(&spans, text, 0, "-- Package docs."),
        Some("comment.documentation")
    );
    assert_eq!(
        name_of(&spans, text, 2, "-- Error docs."),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 3, "Error"), Some("type.definition"));
    assert_eq!(
        name_of(&spans, text, 3, "exception"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 4, "Failure"), Some("type.definition"));
    assert_eq!(
        name_of(&spans, text, 4, "renames"),
        Some("keyword.modifier")
    );
    assert_eq!(name_of(&spans, text, 10, "Outer"), Some("label"));
    assert_eq!(name_of(&spans, text, 10, "loop"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 11, "exit"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 12, "end"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 13, "raise"),
        Some("keyword.exception")
    );
    assert_eq!(name_of(&spans, text, 13, "Error"), Some("type"));
    assert_eq!(name_of(&spans, text, 13, "with"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 15, "when"), Some("keyword.exception"));
    assert_eq!(name_of(&spans, text, 15, "Error"), Some("type"));
}

#[test]
fn highlights_ada_control_flow_and_variants() {
    let text = r#"Outer : loop
   for Index in reverse 1 .. 10 loop
      exit Outer when Index = 5;
   end loop;
end loop Outer;

case Value is
   when 0 => null;
   when others => null;
end case;

type Shape (Kind : Kind_Type) is record
   case Kind is
      when Circle =>
         Radius : Float;
      when others =>
         null;
   end case;
end record;
"#;
    let spans = highlight(LanguageId::Ada, text, 0, 17);
    assert_eq!(name_of(&spans, text, 0, "Outer"), Some("label"));
    assert_eq!(name_of(&spans, text, 1, "for"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 1, "in"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 1, "reverse"), Some("keyword.repeat"));
    assert_eq!(name_of(&spans, text, 2, "exit"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 2, "when"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 3, "end"), Some("keyword.repeat"));
    assert_eq!(
        name_of(&spans, text, 6, "case"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 7, "when"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 9, "end"), Some("keyword.conditional"));
    assert_eq!(name_of(&spans, text, 11, "Shape"), Some("type.definition"));
    assert_eq!(name_of(&spans, text, 11, "record"), Some("keyword.type"));
    assert_eq!(name_of(&spans, text, 12, "is"), Some("keyword.conditional"));
    assert_eq!(
        name_of(&spans, text, 13, "when"),
        Some("keyword.conditional")
    );
    assert_eq!(
        name_of(&spans, text, 17, "end"),
        Some("keyword.conditional")
    );
    assert_eq!(name_of(&spans, text, 18, "record"), Some("keyword.type"));
}
