; Adapted from nvim-treesitter's Lua highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

; Keywords
"return" @keyword.return

[
  "goto"
  "in"
  "local"
  "global"
] @keyword

(break_statement) @keyword

(do_statement
  [
    "do"
    "end"
  ] @keyword)

(while_statement
  [
    "while"
    "do"
    "end"
  ] @keyword.repeat)

(repeat_statement
  [
    "repeat"
    "until"
  ] @keyword.repeat)

(if_statement
  [
    "if"
    "elseif"
    "else"
    "then"
    "end"
  ] @keyword.conditional)

(elseif_statement
  [
    "elseif"
    "then"
  ] @keyword.conditional)

(else_statement
  "else" @keyword.conditional)

(for_statement
  [
    "for"
    "do"
    "end"
  ] @keyword.repeat)

(function_declaration
  [
    "function"
    "end"
  ] @keyword.function)

(function_definition
  [
    "function"
    "end"
  ] @keyword.function)

; Operators
(binary_expression
  operator: _ @operator)

(unary_expression
  operator: _ @operator)

"=" @operator

[
  "and"
  "not"
  "or"
] @keyword.operator

; Punctuation
[
  ";"
  ":"
  "::"
  ","
  "."
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

(table_constructor
  [
    "{"
    "}"
  ] @constructor)

; Comments
(comment) @comment

((comment) @comment.documentation
  (#match? @comment.documentation "^---"))

((comment) @comment.documentation
  (#match? @comment.documentation "^--[ \t]?@"))

; Literals
(hash_bang_line) @keyword.directive

(number) @number

(string) @string

(escape_sequence) @string.escape

; Only the pattern argument of string.find/match/gmatch/gsub is a regexp.
(function_call
  (dot_index_expression
    field: (identifier) @_method
    (#any-of? @_method "find" "match" "gmatch" "gsub"))
  arguments: (arguments
    .
    (_)
    .
    (string
      content: (string_content) @string.regexp)))

(function_call
  (method_index_expression
    method: (identifier) @_method
    (#any-of? @_method "find" "match" "gmatch" "gsub"))
  arguments: (arguments
    .
    (string
      content: (string_content) @string.regexp)))
