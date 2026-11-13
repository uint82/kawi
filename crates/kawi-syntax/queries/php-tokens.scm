; Adapted from nvim-treesitter's php_only highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

[
  "and"
  "as"
  "instanceof"
  "or"
  "xor"
] @keyword.operator

[
  "fn"
  "function"
] @keyword.function

[
  "clone"
  "declare"
  "default"
  "echo"
  "enddeclare"
  "extends"
  "global"
  "goto"
  "implements"
  "insteadof"
  "print"
  "new"
  "unset"
] @keyword

[
  "enum"
  "class"
  "interface"
  "namespace"
  "trait"
] @keyword.type

[
  "abstract"
  "const"
  "final"
  "private"
  "protected"
  "public"
  "readonly"
  "static"
] @keyword.modifier

[
  "return"
  "exit"
  "yield"
  "yield from"
] @keyword.return

[
  "case"
  "else"
  "elseif"
  "endif"
  "endswitch"
  "if"
  "switch"
  "match"
  "??"
] @keyword.conditional

[
  "break"
  "continue"
  "do"
  "endfor"
  "endforeach"
  "endwhile"
  "for"
  "foreach"
  "while"
] @keyword.repeat

[
  "catch"
  "finally"
  "throw"
  "try"
] @keyword.exception

[
  "include_once"
  "include"
  "require_once"
  "require"
  "use"
] @keyword.import

[
  ","
  ";"
  ":"
  "\\"
] @punctuation.delimiter

[
  (php_tag)
  (php_end_tag)
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "#["
] @punctuation.bracket

[
  "="
  "."
  "-"
  "*"
  "/"
  "+"
  "%"
  "**"
  "~"
  "|"
  "^"
  "&"
  "<<"
  ">>"
  "<<<"
  "->"
  "?->"
  "=>"
  "<"
  "<="
  ">="
  ">"
  "<>"
  "<=>"
  "=="
  "!="
  "==="
  "!=="
  "!"
  "&&"
  "||"
  ".="
  "-="
  "+="
  "*="
  "/="
  "%="
  "**="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "??="
  "--"
  "++"
  "@"
  "::"
] @operator

[
  (string)
  (encapsed_string)
  (heredoc_body)
  (nowdoc_body)
  (shell_command_expression)
] @string

(escape_sequence) @string.escape

[
  (heredoc_start)
  (heredoc_end)
] @label

(nowdoc
  "'" @label)

(boolean) @boolean

(null) @constant.builtin

(integer) @number

(float) @number.float

(comment) @comment

((comment) @comment.documentation
  (#match? @comment.documentation "^/[*][*]"))
