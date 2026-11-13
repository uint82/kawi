; Adapted from nvim-treesitter's YAML highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

; Capture keys on their scalar child so @property overrides the scalar highlight.
(comment) @comment

[
  (yaml_directive)
  (tag_directive)
  (reserved_directive)
] @keyword.directive

(tag) @type

[
  (anchor_name)
  (alias_name)
] @label

[
  ","
  "-"
  ":"
  "?"
] @punctuation.delimiter

[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "*"
  "&"
  "---"
  "..."
] @punctuation.special

(boolean_scalar) @boolean

(null_scalar) @constant.builtin

(double_quote_scalar) @string

(single_quote_scalar) @string

(string_scalar) @string

(escape_sequence) @string.escape

(integer_scalar) @number

(float_scalar) @number

(block_scalar) @string

(block_mapping_pair
  key: (flow_node
    [
      (double_quote_scalar) @property
      (single_quote_scalar) @property
      (plain_scalar (_) @property)
    ]))

(flow_pair
  key: (flow_node
    [
      (double_quote_scalar) @property
      (single_quote_scalar) @property
      (plain_scalar (_) @property)
    ]))
