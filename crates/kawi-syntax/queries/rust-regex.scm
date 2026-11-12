; Forked from https://github.com/tree-sitter/tree-sitter-rust
; Copyright (c) 2017 Maxim Sokolov
; Licensed under the MIT license.

; Regex patterns are highlighted separately so they do not look like plain strings.

(call_expression
  function: (scoped_identifier
    path: (identifier) @_regex
    (#any-of? @_regex "Regex" "ByteRegexBuilder")
    name: (identifier) @_new
    (#eq? @_new "new"))
  arguments: (arguments
    (raw_string_literal
      (string_content) @string.regexp)))

(call_expression
  function: (scoped_identifier
    path: (scoped_identifier
      (identifier) @_regex
      (#any-of? @_regex "Regex" "ByteRegexBuilder") .)
    name: (identifier) @_new
    (#eq? @_new "new"))
  arguments: (arguments
    (raw_string_literal
      (string_content) @string.regexp)))

(call_expression
  function: (scoped_identifier
    path: (identifier) @_regex
    (#any-of? @_regex "RegexSet" "RegexSetBuilder")
    name: (identifier) @_new
    (#eq? @_new "new"))
  arguments: (arguments
    (array_expression
      (raw_string_literal
        (string_content) @string.regexp))))

(call_expression
  function: (scoped_identifier
    path: (scoped_identifier
      (identifier) @_regex
      (#any-of? @_regex "RegexSet" "RegexSetBuilder") .)
    name: (identifier) @_new
    (#eq? @_new "new"))
  arguments: (arguments
    (array_expression
      (raw_string_literal
        (string_content) @string.regexp))))
