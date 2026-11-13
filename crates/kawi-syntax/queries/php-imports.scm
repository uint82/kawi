; Adapted from nvim-treesitter's php_only highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

(namespace_use_declaration
  (namespace_use_clause
    [
      (name) @type
      (qualified_name
        (name) @type)
      alias: (name) @type.definition
    ]))

(namespace_use_declaration
  (namespace_use_clause
    type: "function"
    [
      (name) @function
      (qualified_name
        (name) @function)
      alias: (name) @function
    ]))

(namespace_use_declaration
  (namespace_use_clause
    type: "const"
    [
      (name) @constant
      (qualified_name
        (name) @constant)
      alias: (name) @constant
    ]))

(namespace_use_declaration
  body: (namespace_use_group
    (namespace_use_clause
      [
        (name) @type
        (qualified_name
          (name) @type)
        alias: (name) @type.definition
      ])))

(namespace_use_declaration
  type: "function"
  body: (namespace_use_group
    (namespace_use_clause
      [
        (name) @function
        (qualified_name
          (name) @function)
        alias: (name) @function
      ])))

(namespace_use_declaration
  type: "const"
  body: (namespace_use_group
    (namespace_use_clause
      [
        (name) @constant
        (qualified_name
          (name) @constant)
        alias: (name) @constant
      ])))
