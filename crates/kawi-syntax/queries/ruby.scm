(identifier) @variable
(constant) @constant
((constant) @type
 (#match? @type "^[A-Z][A-Z0-9_]*[a-z]"))
"return" @keyword.return
"def" @keyword.function
(call method: [(identifier) (constant)] @function.call)
(call
  receiver: (constant) @type
  method: [(identifier) (constant)] @function.call)
(program
  (call
    (identifier) @keyword.import)
  (#any-of? @keyword.import "require" "require_relative" "load"))
(alias (identifier) @function)
(setter (identifier) @function)
(method name: [(identifier) (constant)] @function)
(singleton_method name: [(identifier) (constant)] @function)
((identifier) @keyword
 (#match? @keyword "^(private|protected|public)$"))
((identifier) @constant.builtin
 (#match? @constant.builtin "^__(FILE|LINE|ENCODING)__$"))
(block_parameter (identifier) @variable.parameter)
(block_parameters (identifier) @variable.parameter)
(destructured_parameter (identifier) @variable.parameter)
(hash_splat_parameter (identifier) @variable.parameter)
(lambda_parameters (identifier) @variable.parameter)
(method_parameters (identifier) @variable.parameter)
(splat_parameter (identifier) @variable.parameter)
(keyword_parameter name: (identifier) @variable.parameter)
(optional_parameter name: (identifier) @variable.parameter)
(regex) @string.regexp
(escape_sequence) @string.escape
