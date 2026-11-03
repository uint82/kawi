[
  "while"
  "repeat"
  "until"
  "for"
  "do"
  "end"
] @keyword.control

[
  "if"
  "elseif"
  "else"
  "then"
] @keyword.control

(field
  name: (identifier) @property)

(dot_index_expression
  field: (identifier) @property)

(parameters
  (identifier) @variable.parameter)

(method_index_expression
  method: (identifier) @function.method)

(hash_bang_line) @keyword.directive
