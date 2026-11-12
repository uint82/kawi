(type_identifier) @type

[
  (type_spec
    name: (type_identifier) @type.definition)
  (type_alias
    name: (type_identifier) @type.definition)
]

(field_identifier) @property

(identifier) @variable

(package_identifier) @module

(parameter_declaration
  (identifier) @variable.parameter)

(variadic_parameter_declaration
  (identifier) @variable.parameter)

(type_parameter_list
  (type_parameter_declaration
    name: (identifier) @type.parameter))

(label_name) @label

(const_spec
  name: (identifier) @constant)

(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method.call))

(function_declaration
  name: (identifier) @function)

(method_declaration
  name: (field_identifier) @function.method)

(method_elem
  name: (field_identifier) @function.method)

; Go does not have constructors, so New and Make prefixes are the convention.
((call_expression
  (identifier) @constructor)
  (#match? @constructor "^([nN]ew|[mM]ake).+$"))

[
  "--"
  "-"
  "-="
  ":="
  "!"
  "!="
  "..."
  "*"
  "*="
  "/"
  "/="
  "&"
  "&&"
  "&="
  "&^"
  "&^="
  "%"
  "%="
  "^"
  "^="
  "+"
  "++"
  "+="
  "<-"
  "<"
  "<<"
  "<<="
  "<="
  "="
  "=="
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "|="
  "||"
  "~"
] @operator

[
  "break"
  "const"
  "continue"
  "default"
  "defer"
  "goto"
  "range"
  "select"
  "var"
  "fallthrough"
] @keyword

[
  "type"
  "struct"
  "interface"
] @keyword.type

"func" @keyword.function

"return" @keyword.return

"go" @keyword.coroutine

"for" @keyword.repeat

[
  "import"
  "package"
] @keyword.import

[
  "else"
  "case"
  "switch"
  "if"
] @keyword.conditional

[
  "chan"
  "map"
] @type.builtin

((type_identifier) @type.builtin
  (#any-of? @type.builtin
    "any" "bool" "byte" "comparable" "complex128" "complex64" "error" "float32" "float64" "int"
    "int16" "int32" "int64" "int8" "rune" "string" "uint" "uint16" "uint32" "uint64" "uint8"
    "uintptr"))

((identifier) @function.builtin
  (#any-of? @function.builtin
    "append" "cap" "clear" "close" "complex" "copy" "delete" "imag" "len" "make" "max" "min" "new"
    "panic" "print" "println" "real" "recover"))

[
  "."
  ","
  ":"
  ";"
] @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

[
  (interpreted_string_literal)
  (raw_string_literal)
] @string

(rune_literal) @character

(escape_sequence) @string.escape

[
  (int_literal)
  (imaginary_literal)
] @number

(float_literal) @number.float

[
  (true)
  (false)
] @boolean

[
  (nil)
  (iota)
] @constant.builtin

(keyed_element
  .
  (literal_element
    (identifier) @variable.member))

(field_declaration
  name: (field_identifier) @variable.member)

(comment) @comment

; Go has no doc comment syntax, so a block above a declaration is the closest signal.
(source_file
  .
  (comment)+ @comment.documentation)

(source_file
  (comment)+ @comment.documentation
  .
  [
    (const_declaration)
    (function_declaration)
    (type_declaration)
    (var_declaration)
  ])
