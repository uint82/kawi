; Adapted from nvim-treesitter's php_only highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

(variable_name) @variable

((name) @constant
  (#match? @constant "^_?[A-Z][A-Z0-9_]*$"))

((name) @constant.builtin
  (#match? @constant.builtin "^__[A-Z][A-Z0-9_]+__$"))

(const_declaration
  (const_element
    (name) @constant))

(enum_case
  name: (name) @constant)

[
  (primitive_type)
  (cast_type)
  (bottom_type)
] @type.builtin

(named_type
  [
    (name) @type
    (qualified_name
      (name) @type)
    (relative_name
      (name) @type)
  ])

(named_type
  (name) @type.builtin
  (#any-of? @type.builtin "static" "self" "parent"))

; Declaration names are definitions so themes can separate them from type uses.
(class_declaration
  name: (name) @type.definition)

(interface_declaration
  name: (name) @type.definition)

(trait_declaration
  name: (name) @type.definition)

(enum_declaration
  name: (name) @type.definition)

(base_clause
  [
    (name) @type
    (qualified_name
      (name) @type)
    (relative_name
      (name) @type)
  ])

(class_interface_clause
  [
    (name) @type
    (qualified_name
      (name) @type)
    (relative_name
      (name) @type)
  ])

(scoped_call_expression
  scope: [
    (name) @type
    (qualified_name
      (name) @type)
    (relative_name
      (name) @type)
  ])

(class_constant_access_expression
  .
  [
    (name) @type
    (qualified_name
      (name) @type)
    (relative_name
      (name) @type)
  ]
  (name) @constant)

(scoped_property_access_expression
  scope: [
    (name) @type
    (qualified_name
      (name) @type)
    (relative_name
      (name) @type)
  ])

(scoped_property_access_expression
  name: (variable_name
    (name) @property))

(use_declaration
  (name) @type)

(binary_expression
  operator: "instanceof"
  right: [
    (name) @type
    (qualified_name
      (name) @type)
    (relative_name
      (name) @type)
  ])

; Functions, methods, constructors
(array_creation_expression
  "array" @function.builtin)

(list_literal
  "list" @function.builtin)

(exit_statement
  "exit" @function.builtin
  "(")

(method_declaration
  name: (name) @function.method)

(function_call_expression
  function: [
    (name) @function.call
    (qualified_name
      (name) @function.call)
    (relative_name
      (name) @function.call)
  ])

(scoped_call_expression
  name: (name) @function.call)

(member_call_expression
  name: (name) @function.method.call)

(nullsafe_member_call_expression
  name: (name) @function.method.call)

(function_definition
  name: (name) @function)

(use_instead_of_clause
  (class_constant_access_expression
    (_)
    (name) @function.method)
  (name) @type)

(use_as_clause
  (class_constant_access_expression
    (_)
    (name) @function.method)*
  (name) @function.method)

(method_declaration
  name: (name) @constructor
  (#eq? @constructor "__construct"))

(object_creation_expression
  [
    (name) @constructor
    (qualified_name
      (name) @constructor)
    (relative_name
      (name) @constructor)
  ])

; Parameters
(variadic_parameter
  "..." @operator
  name: (variable_name) @variable.parameter)

(simple_parameter
  name: (variable_name) @variable.parameter)

(property_promotion_parameter
  name: (variable_name) @variable.parameter)

; Only named arguments are parameters; positional names can be constants.
(argument
  name: (name) @variable.parameter)

; Member access uses @property to stay consistent with the existing PHP capture.
(property_element
  (variable_name) @property)

(member_access_expression
  name: (variable_name
    (name) @property))

(member_access_expression
  name: (name) @property)

(nullsafe_member_access_expression
  name: (variable_name
    (name) @property))

(nullsafe_member_access_expression
  name: (name) @property)

; Variables
(relative_scope) @variable.builtin

((variable_name) @variable.builtin
  (#eq? @variable.builtin "$this"))

; Namespace
(namespace_name
  (name) @module)

(relative_name
  "namespace" @module.builtin)

; Attributes
(attribute_list) @attribute

; Conditions
(conditional_expression
  "?" @keyword.conditional.ternary
  ":" @keyword.conditional.ternary)

; Directives
(declare_directive
  [
    "strict_types"
    "ticks"
    "encoding"
  ] @variable.parameter)

; Labels
(named_label_statement
  (name) @label)

(goto_statement
  (name) @label)

(property_hook
  (name) @label)

(visibility_modifier
  (operation) @label)
