(package_identifier) @namespace
(const_spec name: (identifier) @constant)
(type_spec name: (type_identifier) @type.definition)
(parameter_declaration (identifier) @variable.parameter)
(variadic_parameter_declaration (identifier) @variable.parameter)
(type_parameter_list (type_parameter_declaration name: (identifier) @type.parameter))
((type_identifier) @type.builtin
 (#any-of? @type.builtin
  "any" "bool" "byte" "comparable" "complex64" "complex128" "error" "float32"
  "float64" "int" "int8" "int16" "int32" "int64" "rune" "string" "uint"
  "uint8" "uint16" "uint32" "uint64" "uintptr"))
(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method))
(keyed_element . (literal_element (identifier) @variable.member))
(field_declaration name: (field_identifier) @variable.member)
(function_declaration name: (identifier) @function)
(method_declaration name: (field_identifier) @function.method)
(method_elem name: (field_identifier) @function.method)
(escape_sequence) @string.escape
