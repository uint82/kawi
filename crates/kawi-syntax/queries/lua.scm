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
(function_declaration
  name: (dot_index_expression
    field: (identifier) @function))
(function_declaration
  name: (method_index_expression
    method: (identifier) @function.method))
(function_call
  name: (dot_index_expression
    field: (identifier) @function.call))
(function_call
  name: (method_index_expression
    method: (identifier) @function.method))
(assignment_statement
  (variable_list
    .
    name: (dot_index_expression
      field: (identifier) @function))
  (expression_list
    .
    value: (function_definition)))
(table_constructor
  (field
    name: (identifier) @function
    value: (function_definition)))
