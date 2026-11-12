(function_declarator
  (qualified_identifier
    (identifier) @function))

(function_declarator
  (qualified_identifier
    (qualified_identifier
      (identifier) @function)))

(function_declarator
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (identifier) @function))))

(function_declarator
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (qualified_identifier
          (identifier) @function)))))

(function_declarator
  (template_function
    (identifier) @function))

(function_declarator
  declarator: (field_identifier) @function.method)

(function_declarator
  (template_method
    (field_identifier) @function.method))

(destructor_name
  (identifier) @function.method)

(operator_name) @function

"operator" @function

"static_assert" @function.builtin

(call_expression
  (qualified_identifier
    (identifier) @function.call))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (identifier) @function.call)))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (identifier) @function.call))))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (qualified_identifier
          (identifier) @function.call)))))

(call_expression
  (template_function
    (identifier) @function.call))

(call_expression
  (qualified_identifier
    (template_function
      (identifier) @function.call)))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (template_function
        (identifier) @function.call))))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (template_function
          (identifier) @function.call)))))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (qualified_identifier
          (template_function
            (identifier) @function.call))))))

(call_expression
  (field_expression
    (field_identifier) @function.method.call))

(call_expression
  (field_expression
    (template_method
      (field_identifier) @function.method.call)))

((function_declarator
  (identifier) @constructor)
  (#match? @constructor "^[A-Z]"))

((call_expression
  function: (identifier) @constructor)
  (#match? @constructor "^[A-Z]"))

((function_declarator
  (qualified_identifier
    (identifier) @constructor))
  (#match? @constructor "^[A-Z]"))

((call_expression
  function: (qualified_identifier
    name: (identifier) @constructor))
  (#match? @constructor "^[A-Z]"))

((call_expression
  function: (field_expression
    field: (field_identifier) @constructor))
  (#match? @constructor "^[A-Z]"))

((field_initializer
  (field_identifier) @constructor
  (argument_list))
  (#match? @constructor "^[A-Z]"))

; This grammar parses named casts as template calls, so match the call name.
((call_expression
  function: (template_function
    name: (identifier) @keyword.operator))
  (#any-of? @keyword.operator
    "static_cast" "dynamic_cast" "const_cast" "reinterpret_cast"))

((call_expression
  function: (identifier) @keyword.operator)
  (#eq? @keyword.operator "typeid"))
