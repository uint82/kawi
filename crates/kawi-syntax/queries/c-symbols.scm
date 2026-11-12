(attribute_specifier
  (argument_list
    (identifier) @variable.builtin))

(attribute_specifier
  (argument_list
    (call_expression
      function: (identifier) @variable.builtin)))

(preproc_def
  name: (_) @constant.macro)

(preproc_call
  directive: (preproc_directive) @_directive
  argument: (_) @constant.macro
  (#eq? @_directive "#undef"))

(preproc_ifdef
  name: (identifier) @constant.macro)

(preproc_elifdef
  name: (identifier) @constant.macro)

(preproc_defined
  (identifier) @constant.macro)

(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (field_expression
    field: (field_identifier) @function.call))

((call_expression
  function: (identifier) @function.builtin)
  (#match? @function.builtin "^__builtin_"))

(function_declarator
  declarator: (identifier) @function)

(function_declarator
  declarator: (parenthesized_declarator
    (pointer_declarator
      declarator: (field_identifier) @function)))

(preproc_function_def
  name: (identifier) @function.macro)

(comment) @comment

((comment) @comment.documentation
  (#match? @comment.documentation "^/[*][*][^*].*[*]/$"))

(parameter_declaration
  declarator: (identifier) @variable.parameter)

(parameter_declaration
  declarator: (array_declarator
    declarator: (identifier) @variable.parameter))

(parameter_declaration
  declarator: (pointer_declarator
    declarator: (identifier) @variable.parameter))

(parameter_declaration
  declarator: (pointer_declarator
    declarator: (pointer_declarator
      declarator: (identifier) @variable.parameter)))

(preproc_params
  (identifier) @variable.parameter)

[
  "__attribute__"
  "__declspec"
  "__based"
  "__cdecl"
  "__clrcall"
  "__stdcall"
  "__fastcall"
  "__thiscall"
  "__vectorcall"
  (ms_pointer_modifier)
  (attribute_declaration)
] @attribute
