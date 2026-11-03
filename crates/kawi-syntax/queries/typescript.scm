[
  (required_parameter
    pattern: (object_pattern
      [
        (shorthand_property_identifier_pattern) @variable.parameter
        (pair_pattern value: (identifier) @variable.parameter)
        (object_assignment_pattern left: (shorthand_property_identifier_pattern) @variable.parameter)
        (pair_pattern value: (assignment_pattern left: (identifier) @variable.parameter))
        (rest_pattern (identifier) @variable.parameter)
      ]))
  (optional_parameter
    pattern: (object_pattern
      [
        (shorthand_property_identifier_pattern) @variable.parameter
        (pair_pattern value: (identifier) @variable.parameter)
        (object_assignment_pattern left: (shorthand_property_identifier_pattern) @variable.parameter)
        (pair_pattern value: (assignment_pattern left: (identifier) @variable.parameter))
        (rest_pattern (identifier) @variable.parameter)
      ]))
]
[
  (required_parameter
    pattern: (array_pattern
      [
        (identifier) @variable.parameter
        (assignment_pattern left: (identifier) @variable.parameter)
        (rest_pattern (identifier) @variable.parameter)
      ]))
  (optional_parameter
    pattern: (array_pattern
      [
        (identifier) @variable.parameter
        (assignment_pattern left: (identifier) @variable.parameter)
        (rest_pattern (identifier) @variable.parameter)
      ]))
]
[
  (required_parameter pattern: (rest_pattern (identifier) @variable.parameter))
  (optional_parameter pattern: (rest_pattern (identifier) @variable.parameter))
]
(type_parameter name: (type_identifier) @type.parameter)
(predefined_type) @type.builtin
