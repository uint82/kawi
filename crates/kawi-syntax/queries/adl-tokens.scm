; Adapted from tree-sitter-adl's highlights query.
; Licensed under MIT; see SOURCES.toml.

[
  "module"
  "struct"
  "union"
  "type"
  "newtype"
  "annotation"
] @keyword

"import" @keyword.import

"=" @operator

(version) @number

(comment) @comment
(docstring) @comment.documentation

(json_string) @string
(json_number) @number
(json_object_pair
  (json_string) @property)
(json_value
  ["null" "true" "false"] @constant.builtin)

["{" "}" "[" "]" "<" ">"] @punctuation.bracket
["," ";" "." "::" ":"] @punctuation.delimiter
