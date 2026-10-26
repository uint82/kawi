(default_parameter name: (identifier) @variable.parameter)
(typed_default_parameter name: (identifier) @variable.parameter)
(typed_parameter (identifier) @variable.parameter)
(lambda_parameters (identifier) @variable.parameter)
(type (identifier) @type.builtin
 (#any-of? @type.builtin
  "bool" "bytearray" "bytes" "complex" "dict" "float" "frozenset" "int" "list"
  "memoryview" "object" "range" "set" "str" "tuple" "type"))
(import_statement "import" @keyword.import)
(import_from_statement "from" @keyword.import "import" @keyword.import)
(return_statement "return" @keyword.return)
(function_definition "def" @keyword.function)
((identifier) @variable.builtin
 (#any-of? @variable.builtin "self" "cls"))
