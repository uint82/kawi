(object_reference
  name: (identifier) @type)

(invocation
  (object_reference
    name: (identifier) @function.call))

(cte
  (identifier) @type)

(relation
  alias: (identifier) @variable)

(pivot
  alias: (identifier) @variable)

(unpivot
  alias: (identifier) @variable)

(term
  alias: (identifier) @variable)

(field
  name: (identifier) @variable.member)

(path_expression
  key: (identifier) @variable.member)

(column_definition
  name: (identifier) @variable.member)

(column
  (identifier) @variable.member)

(rename_columns
  name: (identifier) @variable.member)

(rename_columns
  alias: (identifier) @variable)

((term
   value: (cast
    name: (keyword_cast) @function.call
    parameter: [(literal)]?)))

[
  (keyword_gist)
  (keyword_btree)
  (keyword_hash)
  (keyword_spgist)
  (keyword_gin)
  (keyword_brin)
  (keyword_array)
  (keyword_object_id)
] @function.call

(literal) @string
(comment) @comment
(marginalia) @comment

((literal) @number
   (#match? @number "^[-+]?[0-9]+$"))

((literal) @number.float
  (#match? @number.float "^[-+]?([0-9]+\\.[0-9]*|\\.[0-9]+)([eE][-+]?[0-9]+)?$|^[-+]?[0-9]+[eE][-+]?[0-9]+$"))

(parameter) @variable.parameter

(named_argument
  name: (identifier) @variable.parameter)

[
 (keyword_true)
 (keyword_false)
] @boolean
