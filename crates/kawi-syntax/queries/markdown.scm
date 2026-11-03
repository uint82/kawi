(atx_heading
  (inline) @type)

(setext_heading
  (paragraph) @type)

[
  (link_title)
  (indented_code_block)
  (fenced_code_block)
] @string

(link_destination) @string.special

(link_label) @string.special
[
  (atx_h1_marker)
  (atx_h2_marker)
  (atx_h3_marker)
  (atx_h4_marker)
  (atx_h5_marker)
  (atx_h6_marker)
  (setext_h1_underline)
  (setext_h2_underline)
] @punctuation.special
(fenced_code_block_delimiter) @punctuation.delimiter
(code_fence_content) @string
[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
  (list_marker_dot)
  (list_marker_parenthesis)
  (thematic_break)
] @punctuation.special
[
  (block_continuation)
  (block_quote_marker)
] @punctuation.special
(backslash_escape) @string.escape
