(required_parameter
  pattern: (object_pattern
    [
      (shorthand_property_identifier_pattern) @variable.parameter
      (pair_pattern value: (identifier) @variable.parameter)
    ]))
(optional_parameter
  pattern: (object_pattern
    [
      (shorthand_property_identifier_pattern) @variable.parameter
      (pair_pattern value: (identifier) @variable.parameter)
    ]))
(required_parameter
  pattern: (array_pattern
    (identifier) @variable.parameter))
(optional_parameter
  pattern: (array_pattern
    (identifier) @variable.parameter))
