(identifier) @variable
(property_identifier) @variable.member
(shorthand_property_identifier) @variable.member
(private_property_identifier) @variable.member

(object_pattern
  (shorthand_property_identifier_pattern) @variable)

(object_pattern
  (object_assignment_pattern
    (shorthand_property_identifier_pattern) @variable))

((identifier) @type
  (#match? @type "^[A-Z]"))

((identifier) @constant
  (#match? @constant "^_*[A-Z][A-Z\\d_]*$"))

((shorthand_property_identifier) @constant
  (#match? @constant "^_*[A-Z][A-Z\\d_]*$"))

((identifier) @variable.builtin
  (#any-of? @variable.builtin "arguments" "module" "console" "window" "document"))

((identifier) @type.builtin
  (#any-of? @type.builtin
    "Object" "Function" "Boolean" "Symbol" "Number" "Math" "Date" "String" "RegExp" "Map" "Set"
    "WeakMap" "WeakSet" "Promise" "Array" "Int8Array" "Uint8Array" "Uint8ClampedArray" "Int16Array"
    "Uint16Array" "Int32Array" "Uint32Array" "Float32Array" "Float64Array" "ArrayBuffer" "DataView"
    "Error" "EvalError" "InternalError" "RangeError" "ReferenceError" "SyntaxError" "TypeError"
    "URIError"))

(statement_identifier) @label

(function_expression
  name: (identifier) @function)

(function_declaration
  name: (identifier) @function)

(generator_function
  name: (identifier) @function)

(generator_function_declaration
  name: (identifier) @function)

(method_definition
  name: [
    (property_identifier)
    (private_property_identifier)
  ] @function.method)

(method_definition
  name: (property_identifier) @constructor
  (#eq? @constructor "constructor"))

(pair
  key: (property_identifier) @function.method
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (member_expression
    property: (property_identifier) @function.method)
  right: [(function_expression) (arrow_function)])

(variable_declarator
  name: (identifier) @function
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (identifier) @function
  right: [(function_expression) (arrow_function)])

(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (member_expression
    property: [
      (property_identifier)
      (private_property_identifier)
    ] @function.method.call))

(call_expression
  function: (await_expression
    (identifier) @function.call))

(call_expression
  function: (await_expression
    (member_expression
      property: [
        (property_identifier)
        (private_property_identifier)
      ] @function.method.call)))

((identifier) @module.builtin
  (#eq? @module.builtin "Intl"))

((identifier) @function.builtin
  (#any-of? @function.builtin
    "eval" "isFinite" "isNaN" "parseFloat" "parseInt" "decodeURI" "decodeURIComponent" "encodeURI"
    "encodeURIComponent" "require"))

(new_expression
  constructor: (identifier) @constructor)

(new_expression
  constructor: (member_expression
    property: (property_identifier) @constructor))

(decorator
  "@" @attribute
  (identifier) @attribute)

(decorator
  "@" @attribute
  (call_expression
    (identifier) @attribute))

(decorator
  "@" @attribute
  (member_expression
    (property_identifier) @attribute))

(decorator
  "@" @attribute
  (call_expression
    (member_expression
      (property_identifier) @attribute)))

[
  (this)
  (super)
] @variable.builtin

((identifier) @variable.builtin
  (#eq? @variable.builtin "self"))

[
  (true)
  (false)
] @boolean

[
  (null)
  (undefined)
] @constant.builtin

(comment) @comment

(html_comment) @comment

((comment) @comment.documentation
  (#match? @comment.documentation "^/[*][*][^*].*[*]/$"))

(hash_bang_line) @keyword.directive

((string_fragment) @keyword.directive
  (#eq? @keyword.directive "use strict"))

(string) @string

(template_string) @string

(escape_sequence) @string.escape

(regex_pattern) @string.regexp

(regex_flags) @character.special

(regex
  "/" @punctuation.bracket)

(number) @number

((identifier) @number
  (#any-of? @number "NaN" "Infinity"))
