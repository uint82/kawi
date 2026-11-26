; Adapted from tree-sitter-adl's highlights query.
; Licensed under MIT; see SOURCES.toml.

(type_parameters
  (identifier) @type.parameter)

((type_expression
  name: (scoped_name) @type.builtin)
 (#any-of? @type.builtin
   "Void" "Bool" "Int8" "Int16" "Int32" "Int64"
   "Word8" "Word16" "Word32" "Word64"
   "Float" "Double" "Json" "Bytes" "String"
   "Vector" "StringMap" "Nullable" "TypeToken"))
