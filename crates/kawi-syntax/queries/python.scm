(parameters (identifier) @variable.parameter)
(default_parameter name: (identifier) @variable.parameter)
(typed_default_parameter name: (identifier) @variable.parameter)
(typed_parameter (identifier) @variable.parameter)
(lambda_parameters (identifier) @variable.parameter)
(list_splat_pattern (identifier) @variable.parameter)
(dictionary_splat_pattern (identifier) @variable.parameter)
(class_definition name: (identifier) @type)
(class_definition superclasses: (argument_list (identifier) @type))
(class_definition superclasses: (argument_list (keyword_argument value: (identifier) @type)))
(type (identifier) @type)
(type (attribute (identifier) @type))
(generic_type (identifier) @type)
(type (subscript value: [
  (identifier) @type
  (attribute (identifier) @type)
]))
(type (subscript subscript: [
  (identifier) @type
  (attribute (identifier) @type)
]))
(type (subscript subscript: (list [
  (identifier) @type
  (attribute (identifier) @type)
])))
(type (subscript subscript: (subscript value: [
  (identifier) @type
  (attribute (identifier) @type)
])))
(type (subscript subscript: (subscript subscript: [
  (identifier) @type
  (attribute (identifier) @type)
])))
(type (binary_operator
  left: [
    (identifier) @type
    (attribute (identifier) @type)
  ]))
(type (binary_operator
  right: [
    (identifier) @type
    (attribute (identifier) @type)
  ]))
((identifier) @type.builtin
 (#any-of? @type.builtin
  "bool" "bytearray" "bytes" "complex" "dict" "float" "frozenset" "int" "list"
  "memoryview" "object" "range" "set" "str" "tuple" "type"))
(escape_sequence) @string.escape
(import_statement "import" @keyword.import)
(import_from_statement "from" @keyword.import "import" @keyword.import)
(return_statement "return" @keyword.return)
(function_definition "def" @keyword.function)
((identifier) @variable.builtin
 (#any-of? @variable.builtin "self" "cls"))
