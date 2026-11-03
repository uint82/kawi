(namespace_use_declaration "use" @keyword.import)
"return" @keyword.return
"function" @keyword.function
(class_declaration name: (name) @type.definition)
(interface_declaration name: (name) @type.definition)
(trait_declaration name: (name) @type.definition)
(enum_declaration name: (name) @type.definition)
(base_clause (name) @type)
(simple_parameter name: (variable_name) @variable.parameter)
(variadic_parameter name: (variable_name) @variable.parameter)
