[
  "."
  ";"
] @punctuation.delimiter
(preproc_def name: (identifier) @constant)
(preproc_function_def name: (identifier) @function.macro)
(parameter_declaration declarator: (identifier) @variable.parameter)
(parameter_declaration declarator: (pointer_declarator declarator: (identifier) @variable.parameter))
(enumerator name: (identifier) @constant)
(type_definition declarator: (type_identifier) @type.definition)
(struct_specifier name: (type_identifier) @type.definition)
(union_specifier name: (type_identifier) @type.definition)
(enum_specifier name: (type_identifier) @type.definition)
