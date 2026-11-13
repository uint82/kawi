; Adapted from nvim-treesitter's Lua highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

; Variables
(identifier) @variable

((identifier) @constant.builtin
  (#eq? @constant.builtin "_VERSION"))

((identifier) @variable.builtin
  (#eq? @variable.builtin "self"))

((identifier) @module.builtin
  (#any-of? @module.builtin "_G" "debug" "io" "jit" "math" "os" "package" "string" "table" "utf8"))

; `coroutine` is a library, but nvim-treesitter highlights it as a keyword.
((identifier) @keyword.coroutine
  (#eq? @keyword.coroutine "coroutine"))

; Lua has no constants. Uppercase names are the closest convention.
((identifier) @constant
  (#match? @constant "^[A-Z][A-Z_0-9]*$"))

(variable_list
  (attribute
    "<" @punctuation.bracket
    (identifier) @attribute
    ">" @punctuation.bracket))

; Labels
(label_statement
  (identifier) @label)

(goto_statement
  (identifier) @label)

; Tables
(field
  name: (identifier) @property)

(dot_index_expression
  field: (identifier) @variable.member)

; Functions
(parameters
  (identifier) @variable.parameter)

(vararg_expression) @variable.parameter.builtin

(function_declaration
  name: [
    (identifier) @function
    (dot_index_expression
      field: (identifier) @function)
  ])

(function_declaration
  name: (method_index_expression
    method: (identifier) @function.method))

(assignment_statement
  (variable_list
    .
    name: [
      (identifier) @function
      (dot_index_expression
        field: (identifier) @function)
    ])
  (expression_list
    .
    value: (function_definition)))

(table_constructor
  (field
    name: (identifier) @function
    value: (function_definition)))

(function_call
  name: [
    (identifier) @function.call
    (dot_index_expression
      field: (identifier) @function.call)
    (method_index_expression
      method: (identifier) @function.method.call)
  ])

(function_call
  (identifier) @function.builtin
  (#any-of? @function.builtin
    "assert" "collectgarbage" "dofile" "error" "getfenv" "getmetatable" "ipairs" "load" "loadfile"
    "loadstring" "module" "next" "pairs" "pcall" "print" "rawequal" "rawget" "rawlen" "rawset"
    "require" "select" "setfenv" "setmetatable" "tonumber" "tostring" "type" "unpack" "xpcall"
    "__add" "__band" "__bnot" "__bor" "__bxor" "__call" "__concat" "__div" "__eq" "__gc" "__idiv"
    "__index" "__le" "__len" "__lt" "__metatable" "__mod" "__mul" "__name" "__newindex" "__pairs"
    "__pow" "__shl" "__shr" "__sub" "__tostring" "__unm"))

; Literals
(nil) @constant.builtin

[
  (false)
  (true)
] @boolean
