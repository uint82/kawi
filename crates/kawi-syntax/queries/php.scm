(namespace_use_declaration "use" @keyword.import)
"return" @keyword.return
"function" @keyword.function
(class_declaration name: (name) @type.definition)
(interface_declaration name: (name) @type.definition)
(trait_declaration name: (name) @type.definition)
(enum_declaration name: (name) @type.definition)
(base_clause (name) @type)
(method_declaration name: (name) @function.method)
(variable_name) @variable
((name) @variable.builtin
 (#eq? @variable.builtin "this"))
(property_element (variable_name) @property)
(member_access_expression name: (variable_name (name)) @property)
(member_access_expression name: (name) @property)
(simple_parameter name: (variable_name) @variable.parameter)
(variadic_parameter name: (variable_name) @variable.parameter)
