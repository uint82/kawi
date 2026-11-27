; Keywords
[
  "module"
  "import"
  "struct"
  "union"
  "type"
  "newtype"
  "annotation"
] @keyword

; Declaration and field names
(type_name (identifier) @type)
(field_name (identifier) @property)
(type_parameters (identifier) @type.parameter)

; Version markers (struct X#2)
(version) @number

; Annotations
(annotation_decorator "@" @attribute)
(annotation_decorator name: (scoped_name) @attribute)
(annotation_declaration type: (scoped_name) @attribute)

; Builtin primitive types, matched by name (they are not reserved words)
((scoped_name) @type.builtin
 (#any-of? @type.builtin "Void" "Bool" "Int8" "Int16" "Int32" "Int64"
   "Word8" "Word16" "Word32" "Word64" "Float" "Double" "Json" "Bytes"
   "String" "Vector" "StringMap" "Nullable" "TypeToken"))

; Comments and docstrings
(comment) @comment
(docstring) @comment.documentation

; JSON literals
(json_string) @string
(json_number) @number
(json_object_pair (json_string) @property)
(json_value ["null" "true" "false"] @constant.builtin)

; Punctuation
["{" "}" "[" "]" "<" ">"] @punctuation.bracket
["," ";" "." "::" ":"] @punctuation.delimiter
"=" @operator
