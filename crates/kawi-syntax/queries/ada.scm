; Adapted from nvim-treesitter's Ada highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

(procedure_specification
  name: (_) @function)

(function_specification
  name: (_) @function)

(package_declaration
  name: (_) @module)

(package_body
  name: (_) @module)

(package_renaming_declaration
  name: (_) @module)

(generic_instantiation
  "package"
  name: (_) @module)

(generic_instantiation
  [
    "function"
    "procedure"
  ]
  name: (_) @function)

(generic_renaming_declaration
  "package"
  defining_program_unit_name: (_) @module)

(generic_renaming_declaration
  [
    "function"
    "procedure"
  ]
  defining_program_unit_name: (_) @function)

(entry_declaration
  entry_name: (identifier) @function)

(entry_body
  . (identifier) @function)

(accept_statement
  entry_direct_name: (identifier) @function.call)

(entry_index_specification
  . (identifier) @variable.parameter)

(single_task_declaration
  . (identifier) @variable)

(single_protected_declaration
  . (identifier) @variable)

(object_declaration
  name: (identifier) @variable)

(object_declaration
  name: (identifier) @constant
  "constant")

(number_declaration
  (identifier) @constant)

(object_renaming_declaration
  . (identifier) @variable)

(component_declaration
  (identifier) @property)

(discriminant_specification
  (identifier) @property)

(formal_object_declaration
  name: (identifier) @variable.parameter)

(parameter_specification
  (identifier) @variable.parameter)

(loop_parameter_specification
  (identifier) @variable.parameter)

(iterator_specification
  . (identifier) @variable.parameter)

(iterator_specification
  iterator_name: (_) @variable)

(iterator_parameter_specification
  (identifier) @variable.parameter)

(function_call
  name: (identifier) @function.call)

(function_call
  name: (selected_component
    selector_name: (_) @function.call))

(procedure_call_statement
  name: (identifier) @function.call)

(procedure_call_statement
  name: (selected_component
    selector_name: (_) @function.call))

(generic_instantiation
  "package"
  generic_name: (function_call
    name: (_) @module))

(generic_instantiation
  "package"
  generic_name: (function_call
    name: (selected_component
      selector_name: (_) @module)))

(generic_instantiation
  [
    "function"
    "procedure"
  ]
  generic_name: (function_call
    name: (_) @function))

(range_attribute_designator
  "range" @keyword.repeat)

(reduction_attribute_designator
  (identifier) @attribute)

(attribute_designator) @attribute

(aspect_association
  . (identifier) @attribute)

(label
  statement_identifier: (identifier) @label)

(loop_label
  statement_identifier: (identifier) @label)

(use_clause
  "use" @keyword.import
  "type" @keyword.import)

(with_clause
  "private" @keyword.import)

(with_clause
  "limited" @keyword.import)

(formal_concrete_subprogram_declaration
  "with" @keyword)

(formal_abstract_subprogram_declaration
  "with" @keyword)

(formal_package_declaration
  "with" @keyword)

(formal_package_declaration
  . (identifier) @module)

(formal_package_declaration
  generic_package_name: (_) @module)

(use_clause
  (_) @module)

(with_clause
  (_) @module)

(loop_statement
  "end" @keyword.repeat)

(if_statement
  "end" @keyword.conditional)

(case_statement
  "end" @keyword.conditional)

(exit_statement
  "exit" @keyword.repeat)

(loop_parameter_specification
  "in" @keyword.repeat)

(iterator_specification
  [
    "in"
    "of"
  ] @keyword.repeat)

(raise_statement
  "with" @keyword.exception)

(entry_barrier
  "when" @keyword.conditional)

(case_statement_alternative
  "when" @keyword.conditional)

(case_expression_alternative
  "when" @keyword.conditional)

(variant_part
  "is" @keyword.conditional)

(variant_part
  "end" @keyword.conditional)

(variant
  "when" @keyword.conditional)

(exception_handler
  "when" @keyword.exception)

(guard
  "when" @keyword.conditional)

(exit_statement
  "when" @keyword.conditional)

(subprogram_declaration
  "is" @keyword.function
  "abstract" @keyword.function)

(subprogram_body
  "is" @keyword.function)

(expression_function_declaration
  "is" @keyword.function)

(aspect_specification
  "with" @keyword)

((comment) @comment.documentation
  .
  (non_empty_declarative_part
    . (_)))

((comment) @comment.documentation
  .
  [
    (entry_declaration)
    (exception_declaration)
    (expression_function_declaration)
    (full_type_declaration)
    (object_declaration)
    (package_body)
    (package_declaration)
    (parameter_specification)
    (private_type_declaration)
    (protected_type_declaration)
    (subprogram_body)
    (subprogram_declaration)
    (task_type_declaration)
  ])

(compilation
  .
  (comment)+ @comment.documentation)

(component_list
  (component_declaration)
  .
  (comment) @comment.documentation)

(enumeration_type_definition
  (identifier)
  .
  (comment) @comment.documentation)
