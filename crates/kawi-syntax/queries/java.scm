(identifier) @variable

(underscore_pattern) @character.special

(method_declaration
  name: (identifier) @function.method)

(annotation_type_element_declaration
  name: (identifier) @function.method)

(method_invocation
  name: (identifier) @function.method.call)

(method_reference
  "::"
  .
  (identifier) @function.method)

(method_reference
  "::"
  (type_arguments)
  .
  (identifier) @function.method)

(super) @function.builtin

(formal_parameter
  name: (identifier) @variable.parameter)

(catch_formal_parameter
  name: (identifier) @variable.parameter)

(receiver_parameter
  (identifier) @variable.parameter)

(spread_parameter
  (variable_declarator
    name: (identifier) @variable.parameter))

(inferred_parameters
  (identifier) @variable.parameter)

(lambda_expression
  parameters: (identifier) @variable.parameter)

(constant_declaration
  declarator: (variable_declarator
    name: (identifier) @constant))

(enum_constant
  name: (identifier) @constant)

(field_declaration
  declarator: (variable_declarator
    name: (identifier) @variable.member))

(field_access
  field: (identifier) @variable.member)

(this) @variable.builtin

((identifier) @constant
  (#match? @constant "^[A-Z_][A-Z0-9_]+$"))
