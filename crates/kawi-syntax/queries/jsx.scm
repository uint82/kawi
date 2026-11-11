(jsx_element
  open_tag: (jsx_opening_element
    [
      "<"
      ">"
    ] @tag.delimiter))

(jsx_element
  close_tag: (jsx_closing_element
    [
      "</"
      ">"
    ] @tag.delimiter))

(jsx_self_closing_element
  [
    "<"
    "/>"
  ] @tag.delimiter)

(jsx_attribute
  (property_identifier) @tag.attribute)

(jsx_opening_element
  name: (identifier) @tag)

(jsx_closing_element
  name: (identifier) @tag)

(jsx_self_closing_element
  name: (identifier) @tag)

(jsx_opening_element
  (member_expression
    (identifier) @tag
    (property_identifier) @tag))

(jsx_closing_element
  (member_expression
    (identifier) @tag
    (property_identifier) @tag))

(jsx_self_closing_element
  (member_expression
    (identifier) @tag
    (property_identifier) @tag))

(html_character_reference) @character.special

((jsx_attribute
  (property_identifier) @_attr
  (string
    (string_fragment) @string.special.url))
  (#any-of? @_attr "href" "src"))
