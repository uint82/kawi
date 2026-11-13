[
  "+"
  "-"
  "*"
  "/"
  "%"
  "++"
  "--"
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  ">>>="
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "&&"
  "||"
  "!"
  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"
  ">>>"
  "->"
  "::"
  ":"
] @operator

[
  "assert"
  "default"
  "extends"
  "implements"
  "instanceof"
  "permits"
  "to"
  "with"
] @keyword

[
  "record"
  "class"
  "enum"
  "interface"
  "@interface"
] @keyword.type

(synchronized_statement
  "synchronized" @keyword)

[
  "abstract"
  "final"
  "native"
  "non-sealed"
  "open"
  "private"
  "protected"
  "public"
  "sealed"
  "static"
  "strictfp"
  "transitive"
] @keyword.modifier

(modifiers
  "synchronized" @keyword.modifier)

[
  "transient"
  "volatile"
] @keyword.modifier

[
  "return"
  "yield"
] @keyword.return

"new" @keyword.operator

[
  "if"
  "else"
  "switch"
  "case"
  "when"
] @keyword.conditional

(ternary_expression
  [
    "?"
    ":"
  ] @keyword.conditional.ternary)

(wildcard
  "?" @character.special)

[
  "for"
  "while"
  "do"
  "continue"
  "break"
] @keyword.repeat

[
  "exports"
  "import"
  "module"
  "opens"
  "package"
  "provides"
  "requires"
  "uses"
] @keyword.import

(import_declaration
  (asterisk
    "*" @character.special))

[
  "throw"
  "throws"
  "finally"
  "try"
  "catch"
] @keyword.exception

(labeled_statement
  (identifier) @label)

(break_statement
  (identifier) @label)

(continue_statement
  (identifier) @label)

[
  ";"
  "."
  "..."
  ","
] @punctuation.delimiter

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

(type_arguments
  [
    "<"
    ">"
  ] @punctuation.bracket)

(type_parameters
  [
    "<"
    ">"
  ] @punctuation.bracket)

(string_interpolation
  [
    "\\{"
    "}"
  ] @punctuation.special)

(annotation
  "@" @attribute
  name: (identifier) @attribute)

(marker_annotation
  "@" @attribute
  name: (identifier) @attribute)

(annotation
  "@" @attribute
  name: (scoped_identifier
    name: (identifier) @attribute))

(marker_annotation
  "@" @attribute
  name: (scoped_identifier
    name: (identifier) @attribute))

(string_literal) @string

(escape_sequence) @string.escape

(character_literal) @character

[
  (hex_integer_literal)
  (decimal_integer_literal)
  (octal_integer_literal)
  (binary_integer_literal)
] @number

[
  (decimal_floating_point_literal)
  (hex_floating_point_literal)
] @number.float

[
  (true)
  (false)
] @boolean

(null_literal) @constant.builtin

[
  (line_comment)
  (block_comment)
] @comment

; DOTALL lets the pattern span the lines of a normal Javadoc block.
((block_comment) @comment.documentation
  (#match? @comment.documentation "(?s)^/[*][*][^*].*[*]/$"))

((line_comment) @comment.documentation
  (#match? @comment.documentation "^///[^/]"))

((line_comment) @comment.documentation
  (#match? @comment.documentation "^///$"))
