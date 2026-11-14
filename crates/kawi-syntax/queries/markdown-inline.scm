(code_span) @markup.raw

(emphasis) @markup.italic

(strong_emphasis) @markup.strong

(strikethrough) @markup.strikethrough

[
  (link_label)
  (link_text)
  (link_title)
  (image_description)
] @markup.link.label

[
  (link_destination)
  (uri_autolink)
  (email_autolink)
] @markup.link.url

((shortcut_link
  (link_text)) @markup.link.label)

[
  (backslash_escape)
  (hard_line_break)
] @string.escape

[
  (code_span_delimiter)
  (emphasis_delimiter)
] @punctuation.delimiter

(image
  [
    "!"
    "["
    "]"
    "("
    ")"
  ] @punctuation.delimiter)

(inline_link
  [
    "["
    "]"
    "("
    ")"
  ] @punctuation.delimiter)

(shortcut_link
  [
    "["
    "]"
  ] @punctuation.delimiter)
