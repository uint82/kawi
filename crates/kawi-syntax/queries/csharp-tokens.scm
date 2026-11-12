(preproc_error) @keyword.exception

[
  "#define"
  "#undef"
] @keyword.directive.define

[
  "#if"
  "#elif"
  "#else"
  "#endif"
  "#region"
  "#endregion"
  "#line"
  "#pragma"
  "#nullable"
  "#error"
  (shebang_directive)
] @keyword.directive

[
  (preproc_line)
  (preproc_pragma)
  (preproc_nullable)
] @constant.macro

(preproc_pragma
  (identifier) @constant)

(preproc_if
  (identifier) @constant)

[
  "if"
  "else"
  "switch"
  "break"
  "case"
  "when"
] @keyword.conditional

[
  "while"
  "for"
  "do"
  "continue"
  "goto"
  "foreach"
] @keyword.repeat

[
  "try"
  "catch"
  "throw"
  "finally"
] @keyword.exception

[
  "+"
  "?"
  ":"
  "++"
  "-"
  "--"
  "&"
  "&&"
  "|"
  "||"
  "!"
  "!="
  "=="
  "*"
  "/"
  "%"
  "<"
  "<="
  ">"
  ">="
  "="
  "-="
  "+="
  "*="
  "/="
  "%="
  "^"
  "^="
  "&="
  "|="
  "~"
  ">>"
  ">>>"
  "<<"
  "<<="
  ">>="
  ">>>="
  "=>"
  "??"
  "??="
  ".."
] @operator

(list_pattern
  ".." @character.special)

(discard) @character.special

[
  ";"
  "."
  ","
  ":"
  "::"
] @punctuation.delimiter

(conditional_expression
  [
    "?"
    ":"
  ] @keyword.conditional.ternary)

[
  "["
  "]"
  "{"
  "}"
  "("
  ")"
] @punctuation.bracket

(interpolation_brace) @punctuation.special

(type_parameter_list
  [
    "<"
    ">"
  ] @punctuation.bracket)

(type_argument_list
  [
    "<"
    ">"
  ] @punctuation.bracket)

[
  "using"
  "as"
] @keyword.import

(alias_qualified_name
  (identifier
    "global") @keyword.import)

[
  "with"
  "new"
  "typeof"
  "sizeof"
  "is"
  "and"
  "or"
  "not"
  "stackalloc"
  "__makeref"
  "__reftype"
  "__refvalue"
  "in"
  "out"
  "ref"
] @keyword.operator

[
  "lock"
  "params"
  "operator"
  "default"
  "implicit"
  "explicit"
  "override"
  "get"
  "set"
  "init"
  "where"
  "add"
  "remove"
  "checked"
  "unchecked"
  "fixed"
  "alias"
  "file"
  "unsafe"
] @keyword

(attribute_target_specifier
  .
  _ @keyword)

[
  "enum"
  "record"
  "class"
  "struct"
  "interface"
  "namespace"
  "event"
  "delegate"
] @keyword.type

[
  "async"
  "await"
] @keyword.coroutine

[
  "const"
  "extern"
  "readonly"
  "static"
  "volatile"
  "required"
  "managed"
  "unmanaged"
  "notnull"
  "abstract"
  "private"
  "protected"
  "internal"
  "public"
  "partial"
  "sealed"
  "virtual"
  "global"
] @keyword.modifier

(scoped_type
  "scoped" @keyword.modifier)

(query_expression
  (_
    [
      "from"
      "orderby"
      "select"
      "group"
      "by"
      "ascending"
      "descending"
      "equals"
      "let"
    ] @keyword))

[
  "return"
  "yield"
] @keyword.return
