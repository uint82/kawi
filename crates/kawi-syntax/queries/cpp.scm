(namespace_definition name: (namespace_identifier) @namespace)
(namespace_identifier) @namespace
(type_parameter_declaration (type_identifier) @type.parameter)
(template_argument_list ["<" ">"] @punctuation.bracket)
(template_parameter_list ["<" ">"] @punctuation.bracket)
(class_specifier name: (type_identifier) @type.definition)
(call_expression function: (field_expression field: (field_identifier) @function.method))
(function_declarator declarator: (field_identifier) @function.method)
