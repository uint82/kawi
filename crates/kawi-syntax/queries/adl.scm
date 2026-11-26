; Adapted from tree-sitter-adl's highlights query.
; Licensed under MIT; see SOURCES.toml.

(module_definition
  name: (scoped_name) @module)

(import_declaration
  path: (import_path
    (scoped_name) @module))

(import_path
  ".*" @punctuation.special)

(struct_definition
  name: (type_name) @type.definition)

(union_definition
  name: (type_name) @type.definition)

(type_definition
  name: (type_name) @type.definition)

(newtype_definition
  name: (type_name) @type.definition)

(field
  name: (field_name) @property)

(type_expression
  name: (scoped_name) @type)

(annotation_decorator
  "@" @punctuation.special
  name: (scoped_name) @attribute)

(annotation_declaration
  target: (scoped_name) @type)

(annotation_declaration
  type: (scoped_name) @attribute)
