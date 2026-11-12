((identifier) @variable.member
  (#match? @variable.member "^m_.*$"))

(parameter_declaration
  declarator: (reference_declarator
    (identifier) @variable.parameter))

(parameter_declaration
  declarator: (pointer_declarator
    declarator: (reference_declarator
      (identifier) @variable.parameter)))

(optional_parameter_declaration
  declarator: (identifier) @variable.parameter)

(optional_parameter_declaration
  declarator: (pointer_declarator
    declarator: (identifier) @variable.parameter))

(optional_parameter_declaration
  declarator: (reference_declarator
    (identifier) @variable.parameter))

(optional_parameter_declaration
  declarator: (array_declarator
    declarator: (identifier) @variable.parameter))

(optional_parameter_declaration
  declarator: (pointer_declarator
    declarator: (pointer_declarator
      declarator: (identifier) @variable.parameter)))

(optional_parameter_declaration
  declarator: (pointer_declarator
    declarator: (reference_declarator
      (identifier) @variable.parameter)))

(field_declaration
  (field_identifier) @variable.member)

(field_initializer
  (field_identifier) @property)

(case_statement
  value: (qualified_identifier
    (identifier) @constant))

(concept_definition
  name: (identifier) @type.definition)

(alias_declaration
  name: (type_identifier) @type.definition)

(class_specifier
  name: (type_identifier) @type.definition)

(struct_specifier
  name: (type_identifier) @type.definition)

(union_specifier
  name: (type_identifier) @type.definition)

(enum_specifier
  name: (type_identifier) @type.definition)

(type_parameter_declaration
  (type_identifier) @type.parameter)

(auto) @type.builtin

(namespace_identifier) @module

((namespace_identifier) @type
  (#match? @type "^[A-Z]"))

(using_declaration
  .
  "using"
  .
  "namespace"
  .
  [
    (qualified_identifier)
    (identifier)
  ] @module)

(this) @variable.builtin

(raw_string_literal) @string

[
  "try"
  "catch"
  "noexcept"
  "throw"
] @keyword.exception

[
  "decltype"
  "explicit"
  "friend"
  "override"
  "using"
  "requires"
  "constexpr"
] @keyword

[
  "class"
  "namespace"
  "template"
  "typename"
  "concept"
] @keyword.type

[
  "co_await"
  "co_yield"
  "co_return"
] @keyword.coroutine

[
  "public"
  "private"
  "protected"
  "final"
  "virtual"
] @keyword.modifier

[
  "new"
  "delete"
  "xor"
  "bitand"
  "bitor"
  "compl"
  "not"
  "xor_eq"
  "and_eq"
  "or_eq"
  "not_eq"
  "and"
  "or"
] @keyword.operator

"<=>" @operator

"::" @punctuation.delimiter

(template_argument_list
  [
    "<"
    ">"
  ] @punctuation.bracket)

(template_parameter_list
  [
    "<"
    ">"
  ] @punctuation.bracket)
