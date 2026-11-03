(formal_parameters
  [
    (identifier) @variable.parameter
    (assignment_pattern left: (identifier) @variable.parameter)
    (rest_pattern (identifier) @variable.parameter)
    (array_pattern
      [
        (identifier) @variable.parameter
        (assignment_pattern left: (identifier) @variable.parameter)
        (rest_pattern (identifier) @variable.parameter)
      ])
    (object_pattern
      [
        (pair_pattern value: (identifier) @variable.parameter)
        (pair_pattern value: (assignment_pattern left: (identifier) @variable.parameter))
        (shorthand_property_identifier_pattern) @variable.parameter
        (object_assignment_pattern left: (shorthand_property_identifier_pattern) @variable.parameter)
        (rest_pattern (identifier) @variable.parameter)
      ])
  ])
(formal_parameters
  (assignment_pattern
    left: (array_pattern
      [
        (identifier) @variable.parameter
        (assignment_pattern left: (identifier) @variable.parameter)
        (rest_pattern (identifier) @variable.parameter)
      ])))
(formal_parameters
  (assignment_pattern
    left: (object_pattern
      [
        (pair_pattern value: (identifier) @variable.parameter)
        (pair_pattern value: (assignment_pattern left: (identifier) @variable.parameter))
        (shorthand_property_identifier_pattern) @variable.parameter
        (object_assignment_pattern left: (shorthand_property_identifier_pattern) @variable.parameter)
        (rest_pattern (identifier) @variable.parameter)
      ])))
