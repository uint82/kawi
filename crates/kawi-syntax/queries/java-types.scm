(interface_declaration
  name: (identifier) @type)

(annotation_type_declaration
  name: (identifier) @type)

(class_declaration
  name: (identifier) @type)

(record_declaration
  name: (identifier) @type)

(enum_declaration
  name: (identifier) @type)

(constructor_declaration
  name: (identifier) @type)

(compact_constructor_declaration
  name: (identifier) @type)

(type_identifier) @type

((type_identifier) @type.builtin
  (#eq? @type.builtin "var"))

[
  (boolean_type)
  (integral_type)
  (floating_point_type)
  (void_type)
] @type.builtin

(type_parameter
  (type_identifier) @type.parameter)

((method_invocation
  object: (identifier) @type)
  (#match? @type "^[A-Z]"))

((method_reference
  .
  (identifier) @type)
  (#match? @type "^[A-Z]"))

((field_access
  object: (identifier) @type)
  (#match? @type "^[A-Z]"))

((scoped_identifier
  (identifier) @type)
  (#match? @type "^[A-Z]"))
