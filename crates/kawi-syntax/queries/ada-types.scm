; Adapted from nvim-treesitter's Ada highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

(task_body
  . (identifier) @type)

(protected_body
  . (identifier) @type)

(full_type_declaration
  . (identifier) @type.definition)

(incomplete_type_declaration
  . (identifier) @type.definition)

(private_type_declaration
  . (identifier) @type.definition)

(private_extension_declaration
  . (identifier) @type.definition)

(subtype_declaration
  . (identifier) @type.definition)

(task_type_declaration
  . (identifier) @type.definition)

(protected_type_declaration
  . (identifier) @type.definition)

(formal_complete_type_declaration
  . (identifier) @type.definition)

(formal_incomplete_type_declaration
  . (identifier) @type.definition)

(enumeration_type_definition
  (identifier) @constant)

(exception_declaration
  (identifier) @type.definition)

(exception_renaming_declaration
  . (identifier) @type.definition)

(_
  subtype_mark: (_) @type)

(qualified_expression
  subtype_name: (_) @type)

(function_call
  name: (identifier) @type
  name: (tick)
  name: (attribute_designator))

(function_call
  name: (selected_component
    selector_name: (identifier) @type)
  name: (tick)
  name: (attribute_designator))

(raise_statement
  name: (_) @type)

(exception_choice
  exception_name: (_) @type)

(full_type_declaration
  "type" @keyword.type
  "is" @keyword.type)

(incomplete_type_declaration
  "type" @keyword.type)

(subtype_declaration
  "subtype" @keyword.type
  "is" @keyword.type)

(private_type_declaration
  "type" @keyword.type)

(private_extension_declaration
  "type" @keyword.type)

(task_type_declaration
  "type" @keyword.type)

(protected_type_declaration
  "type" @keyword.type)

(formal_complete_type_declaration
  "type" @keyword.type)

(formal_incomplete_type_declaration
  "type" @keyword.type)

(formal_complete_type_declaration
  "is" @keyword.type)

(formal_private_type_definition
  "private" @keyword.type)

(record_definition
  "end" @keyword.type)

(array_type_definition
  "array" @keyword.type
  "of" @keyword.type)

(access_to_object_definition
  "access" @keyword.type)

(access_to_object_definition
  "access" @keyword.type
  [
    (general_access_modifier
      "constant" @keyword.type)
    (general_access_modifier
      "all" @keyword.type)
  ])

(access_to_subprogram_definition
  "access" @keyword.type)

(range_constraint
  "range" @keyword.type)

(signed_integer_type_definition
  "range" @keyword.type)

(formal_signed_integer_type_definition
  "range" @keyword.type)

(formal_modular_type_definition
  "mod" @keyword.type)

(formal_ordinary_fixed_point_definition
  "delta" @keyword.type)

(formal_decimal_fixed_point_definition
  "delta" @keyword.type)

(formal_floating_point_definition
  "digits" @keyword.type)

(index_subtype_definition
  "range" @keyword.type)

(record_type_definition
  "abstract" @keyword.type)

(record_type_definition
  "tagged" @keyword.type)

(record_type_definition
  "limited" @keyword.type)

(record_type_definition
  (record_definition
    "null" @keyword.type))

(private_type_declaration
  "is" @keyword.type
  "private" @keyword.type)

(private_type_declaration
  "tagged" @keyword.type)

(private_type_declaration
  "limited" @keyword.type)

(task_type_declaration
  "task" @keyword.type
  "is" @keyword.type)

(protected_type_declaration
  "protected" @keyword.type
  "is" @keyword.type)
