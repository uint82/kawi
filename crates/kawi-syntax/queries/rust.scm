; Forked from https://github.com/tree-sitter/tree-sitter-rust
; Copyright (c) 2017 Maxim Sokolov
; Licensed under the MIT license.
; Identifier conventions
(shebang) @keyword.directive

(identifier) @variable

((identifier) @type
  (#match? @type "^[A-Z]"))

(const_item
  name: (identifier) @constant)

; Assume all-caps names are constants
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]*$"))

; Other identifiers
(type_identifier) @type

(primitive_type) @type.builtin

(field_identifier) @variable.member

(shorthand_field_identifier) @variable.member

(shorthand_field_initializer
  (identifier) @variable.member)

(mod_item
  name: (identifier) @module)

(self) @variable.builtin

"_" @character.special

(label
  [
    "'"
    (identifier)
  ] @label)

; Function definitions
(function_item
  (identifier) @function)

(function_signature_item
  (identifier) @function)

(parameter
  [
    (identifier)
    "_"
  ] @variable.parameter)

(parameter
  (ref_pattern
    [
      (mut_pattern
        (identifier) @variable.parameter)
      (identifier) @variable.parameter
    ]))

(closure_parameters
  (_) @variable.parameter)

; Function calls
(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (scoped_identifier
    (identifier) @function.call .))

(call_expression
  function: (field_expression
    field: (field_identifier) @function.call))

(generic_function
  function: (identifier) @function.call)

(generic_function
  function: (scoped_identifier
    name: (identifier) @function.call))

(generic_function
  function: (field_expression
    field: (field_identifier) @function.call))

; Assume other uppercase names are enum constructors
((field_identifier) @constant
  (#match? @constant "^[A-Z]"))

(enum_variant
  name: (identifier) @constant)

; Assume that uppercase names in paths are types
(scoped_identifier
  path: (identifier) @module)

(scoped_identifier
  (scoped_identifier
    name: (identifier) @module))

(scoped_type_identifier
  path: (identifier) @module)

(scoped_type_identifier
  path: (identifier) @type
  (#match? @type "^[A-Z]"))

(scoped_type_identifier
  (scoped_identifier
    name: (identifier) @module))

((scoped_identifier
  path: (identifier) @type)
  (#match? @type "^[A-Z]"))

((scoped_identifier
  name: (identifier) @type)
  (#match? @type "^[A-Z]"))

((scoped_identifier
  name: (identifier) @constant)
  (#match? @constant "^[A-Z][A-Z0-9_]*$"))

((scoped_identifier
  path: (identifier) @type
  name: (identifier) @constant)
  (#match? @type "^[A-Z]")
  (#match? @constant "^[A-Z]"))

((scoped_type_identifier
  path: (identifier) @type
  name: (type_identifier) @constant)
  (#match? @type "^[A-Z]")
  (#match? @constant "^[A-Z]"))

[
  (crate)
  (super)
] @module

(scoped_use_list
  path: (identifier) @module)

(scoped_use_list
  path: (scoped_identifier
    (identifier) @module))

(use_list
  (scoped_identifier
    (identifier) @module
    .
    (_)))

(use_list
  (identifier) @type
  (#match? @type "^[A-Z]"))

(use_as_clause
  alias: (identifier) @type
  (#match? @type "^[A-Z]"))

; Correct enum constructors
(call_expression
  function: (scoped_identifier
    "::"
    name: (identifier) @constant)
  (#match? @constant "^[A-Z]"))

; Assume uppercase names in a match arm are constants.
((match_arm
  pattern: (match_pattern
    (identifier) @constant))
  (#match? @constant "^[A-Z]"))

((match_arm
  pattern: (match_pattern
    (scoped_identifier
      name: (identifier) @constant)))
  (#match? @constant "^[A-Z]"))

((identifier) @constant.builtin
  (#any-of? @constant.builtin "Some" "None" "Ok" "Err"))

; Macro definitions
"$" @function.macro

(metavariable) @function.macro

(macro_definition
  "macro_rules!" @function.macro)

; Attribute macros
(attribute_item
  (attribute
    (identifier) @function.macro))

(inner_attribute_item
  (attribute
    (identifier) @function.macro))

(attribute
  (scoped_identifier
    (identifier) @function.macro .))

; Function-like macros
(macro_invocation
  macro: (identifier) @function.macro)

(macro_invocation
  macro: (scoped_identifier
    (identifier) @function.macro .))

(type_parameters
  (type_parameter name: (type_identifier) @type.parameter))
