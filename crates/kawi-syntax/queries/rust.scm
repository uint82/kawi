(integer_literal) @number
(float_literal) @number
(return_expression "return" @keyword.return)
(use_declaration "use" @keyword.import)
(function_item "fn" @keyword.function)
(shorthand_field_identifier) @variable.member
(shorthand_field_initializer (identifier) @variable.member)
(type_parameters (type_parameter name: (type_identifier) @type.parameter))
