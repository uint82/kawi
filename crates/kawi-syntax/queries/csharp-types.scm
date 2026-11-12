(property_declaration
  name: (identifier) @property)

(property_declaration
  type: (identifier) @type)

(nullable_type
  type: (identifier) @type)

(array_type
  type: (identifier) @type)

(ref_type
  type: (identifier) @type)

(pointer_type
  type: (identifier) @type)

(catch_declaration
  type: (identifier) @type)

(interface_declaration
  name: (identifier) @type)

(class_declaration
  name: (identifier) @type)

(record_declaration
  name: (identifier) @type)

(struct_declaration
  name: (identifier) @type)

(enum_declaration
  name: (identifier) @type)

(delegate_declaration
  name: (identifier) @type)

(enum_member_declaration
  name: (identifier) @variable.member)

(operator_declaration
  type: (identifier) @type)

(conversion_operator_declaration
  type: (identifier) @type)

(explicit_interface_specifier
  [
    (identifier) @type
    (generic_name
      (identifier) @type)
  ])

(primary_constructor_base_type
  type: (identifier) @type)

[
  "assembly"
  "module"
  "this"
  "base"
] @variable.builtin

(constructor_declaration
  name: (identifier) @constructor)

(destructor_declaration
  name: (identifier) @constructor)

(constructor_initializer
  "base" @constructor)

(variable_declaration
  (identifier) @type)

(object_creation_expression
  (identifier) @type)

(typeof_expression
  (generic_name
    (identifier) @type))

(type_argument_list
  (generic_name
    (identifier) @type))

(base_list
  (generic_name
    (identifier) @type))

(type_parameter_constraint
  [
    (identifier) @type
    (type
      (generic_name
        (identifier) @type))
  ])

(object_creation_expression
  (generic_name
    (identifier) @type))

(property_declaration
  (generic_name
    (identifier) @type))

(_
  type: (generic_name
    (identifier) @type))

(base_list
  (identifier) @type)

(type_argument_list
  (identifier) @type)

(type_parameter_list
  (type_parameter) @type.parameter)

(type_parameter
  name: (identifier) @type.parameter)

(type_parameter_constraints_clause
  "where"
  .
  (identifier) @type.parameter)

(attribute
  name: (identifier) @attribute)

(foreach_statement
  type: (identifier) @type)

(goto_statement
  (identifier) @label)

(labeled_statement
  (identifier) @label)

(tuple_element
  type: (identifier) @type)

(tuple_expression
  (argument
    (declaration_expression
      type: (identifier) @type)))

(cast_expression
  type: (identifier) @type)

(lambda_expression
  type: (identifier) @type)

(lambda_expression
  (modifier) @keyword.modifier)

(as_expression
  right: (identifier) @type)

(typeof_expression
  (identifier) @type)
