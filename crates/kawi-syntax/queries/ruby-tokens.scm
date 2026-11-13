; Adapted from nvim-treesitter's ruby highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

[
  "alias"
  "begin"
  "do"
  "end"
  "module"
] @keyword

[
  "BEGIN"
  "END"
] @keyword

"class" @keyword.type

[
  "return"
  "yield"
] @keyword.return

[
  "and"
  "in"
  "not"
  "or"
] @keyword.operator

[
  "def"
  "undef"
] @keyword.function

[
  "case"
  "else"
  "elsif"
  "if"
  "then"
  "unless"
  "when"
] @keyword.conditional

(in_clause
  "in" @keyword.conditional)

[
  "break"
  "for"
  "next"
  "redo"
  "retry"
  "until"
  "while"
] @keyword.repeat

(in
  "in" @keyword.repeat)

[
  "ensure"
  "rescue"
] @keyword.exception

(method
  "end" @keyword.function)

(singleton_method
  "end" @keyword.function)

(if
  "end" @keyword.conditional)

"defined?" @function.builtin

[
  (string)
  (bare_string)
  (subshell)
  (heredoc_body)
  (chained_string)
] @string

[
  (heredoc_beginning)
  (heredoc_end)
] @label

[
  (simple_symbol)
  (delimited_symbol)
  (hash_key_symbol)
  (bare_symbol)
] @string.special.symbol

(regex) @string.regexp

(escape_sequence) @string.escape

(integer) @number

(float) @number.float

(complex) @number

(rational) @number

(character) @character

[
  (true)
  (false)
] @boolean

(nil) @constant.builtin

(hash_splat_nil) @constant.builtin

(comment) @comment

(program
  (comment)+ @comment.documentation
  (class))

(module
  (comment)+ @comment.documentation
  (body_statement
    (class)))

(class
  (comment)+ @comment.documentation
  (body_statement
    (method)))

(body_statement
  (comment)+ @comment.documentation
  (method))

((program
  .
  (comment) @keyword.directive)
  (#match? @keyword.directive "^#!/"))

[
  "!"
  "!="
  "!~"
  "%"
  "%="
  "&"
  "&&"
  "&&="
  "&="
  "*"
  "**"
  "**="
  "*="
  "+"
  "+="
  "-"
  "-="
  "->"
  "/"
  "/="
  "<"
  "<<"
  "<<="
  "<="
  "<=>"
  "="
  "=="
  "==="
  "=~"
  ">"
  ">="
  ">>"
  ">>="
  "?"
  "^"
  "^="
  "|"
  "|="
  "||"
  "||="
  "~"
  "=>"
  ".."
  "..."
] @operator

(conditional
  "?" @keyword.conditional.ternary
  ":" @keyword.conditional.ternary)

[
  ","
  ";"
  "."
  "&."
  "::"
] @punctuation.delimiter

(pair
  ":" @punctuation.delimiter)

(keyword_pattern
  ":" @punctuation.delimiter)

(keyword_parameter
  ":" @punctuation.delimiter)

(optional_parameter
  "=" @operator)

(regex
  "/" @punctuation.bracket)

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "%w("
  "%i("
] @punctuation.bracket

(block_parameters
  "|" @punctuation.bracket)

(interpolation
  "#{" @punctuation.special
  "}" @punctuation.special)
