; From tree-sitter-python licensed under MIT License
; Copyright (c) 2016 Max Brunsfeld

; Regex patterns are strings, but they deserve a different color.

; Regex from the `re` module
(call
  function: (attribute
    object: (identifier) @_re)
  arguments: (argument_list
    (string
      (string_content) @string.regexp))
  (#eq? @_re "re"))

(call
  function: (attribute
    object: (identifier) @_re)
  arguments: (argument_list
    (concatenated_string
      (string
        (string_content) @string.regexp)))
  (#eq? @_re "re"))
