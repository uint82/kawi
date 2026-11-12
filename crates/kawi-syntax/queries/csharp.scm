[
  (identifier)
  (preproc_arg)
] @variable

((preproc_arg) @constant.macro
  (#match? @constant.macro "^[_A-Z][_A-Z0-9]*$"))

(method_declaration
  name: (identifier) @function.method)

(local_function_statement
  name: (identifier) @function.method)

(method_declaration
  returns: [
    (identifier) @type
    (generic_name
      (identifier) @type)
  ])

(event_declaration
  type: (identifier) @type)

(event_declaration
  name: (identifier) @variable.member)

(event_field_declaration
  (variable_declaration
    (variable_declarator
      name: (identifier) @variable.member)))

(declaration_pattern
  type: (identifier) @type)

(local_function_statement
  type: (identifier) @type)

(member_access_expression
  name: (identifier) @variable.member)

(invocation_expression
  (member_access_expression
    name: (identifier) @function.method.call))

(conditional_access_expression
  (member_binding_expression
    name: (identifier) @variable.member))

(invocation_expression
  function: (conditional_access_expression
    (member_binding_expression
      name: (identifier) @function.method.call)))

(invocation_expression
  function: (member_access_expression
    name: (generic_name
      (identifier) @function.method.call)))

(invocation_expression
  function: (generic_name
    .
    (identifier) @function.method.call))

(namespace_declaration
  name: (identifier) @module)

(file_scoped_namespace_declaration
  name: (identifier) @module)

(qualified_name
  (identifier) @type)

(invocation_expression
  (identifier) @function.method.call)

(field_declaration
  (variable_declaration
    (variable_declarator
      (identifier) @variable.member)))

(initializer_expression
  (assignment_expression
    left: (identifier) @variable.member))

(parameter
  name: (identifier) @variable.parameter)

(parameter
  (modifier) @keyword.modifier)

(parameter_list
  name: (identifier) @variable.parameter)

(bracketed_parameter_list
  name: (identifier) @variable.parameter)

(implicit_parameter) @variable.parameter

(parameter_list
  (parameter
    type: (identifier) @type))

(integer_literal) @number

(real_literal) @number.float

(null_literal) @constant.builtin

(calling_convention
  [
    (identifier)
    "Cdecl"
    "Stdcall"
    "Thiscall"
    "Fastcall"
  ] @attribute.builtin)

(character_literal) @character

[
  (string_literal)
  (raw_string_literal)
  (verbatim_string_literal)
  (interpolated_string_expression)
] @string

(escape_sequence) @string.escape

[
  "true"
  "false"
] @boolean

(predefined_type) @type.builtin

(implicit_type) @keyword

(comment) @comment

((comment) @comment.documentation
  (#match? @comment.documentation "^(///|/[*][*])"))

(using_directive
  (identifier) @type)

(using_directive
  (type) @type.definition)
