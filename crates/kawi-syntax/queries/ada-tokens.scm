; Adapted from nvim-treesitter's Ada highlights.
; Licensed under Apache-2.0; see SOURCES.toml.
;
; Query literals are case-sensitive; Ada keywords are not. Upstream has the
; same limitation, so uppercase keywords parse but stay unhighlighted.

[
  "abort"
  "abs"
  "abstract"
  "accept"
  "access"
  "all"
  "array"
  "at"
  "begin"
  "body"
  "declare"
  "delay"
  "delta"
  "digits"
  "do"
  "end"
  "entry"
  "exit"
  "generic"
  "interface"
  "is"
  "limited"
  "mod"
  "new"
  "null"
  "of"
  "others"
  "out"
  "overriding"
  "package"
  "pragma"
  "private"
  "protected"
  "range"
  "separate"
  "subtype"
  "synchronized"
  "tagged"
  "task"
  "terminate"
  "type"
  "until"
  "when"
] @keyword

"record" @keyword.type

[
  "aliased"
  "constant"
  "renames"
] @keyword.modifier

[
  "with"
  "use"
] @keyword.import

[
  "function"
  "procedure"
] @keyword.function

[
  "and"
  "in"
  "not"
  "or"
  "xor"
] @keyword.operator

[
  "while"
  "loop"
  "for"
  "parallel"
  "reverse"
  "some"
] @keyword.repeat

"return" @keyword.return

[
  "case"
  "if"
  "else"
  "then"
  "elsif"
  "select"
] @keyword.conditional

[
  "exception"
  "raise"
] @keyword.exception

(comment) @comment

(string_literal) @string

(character_literal) @character

(numeric_literal) @number

[
  (relational_operator)
  (binary_adding_operator)
  (unary_adding_operator)
  (multiplying_operator)
] @operator

[
  "**"
  "=>"
  ":="
  ".."
  "|"
] @operator

(target_name) @operator

(factor_abs
  "abs" @keyword.operator)

(tick) @punctuation.delimiter

[
  ","
  ";"
  "."
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

(gnatprep_identifier) @keyword.directive

(gnatprep_declarative_if_statement) @keyword.directive

(gnatprep_if_statement) @keyword.directive

; Ada Boolean literals are identifiers, not reserved words.
((identifier) @boolean
  (#match? @boolean "(?i)^(true|false)$"))
