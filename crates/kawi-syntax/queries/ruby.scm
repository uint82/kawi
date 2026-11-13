; Adapted from nvim-treesitter's ruby highlights.
; Licensed under Apache-2.0; see SOURCES.toml.

(identifier) @variable
(global_variable) @variable

[
  (class_variable)
  (instance_variable)
] @variable.member

[
  (self)
  (super)
] @variable.builtin

(constant) @constant

((constant) @type
  (#match? @type "^[A-Z][A-Z0-9_]*[a-z]"))

((identifier) @constant.builtin
  (#any-of? @constant.builtin
    "__callee__" "__dir__" "__id__" "__method__" "__send__"
    "__ENCODING__" "__FILE__" "__LINE__"))

((constant) @constant.builtin
  (#any-of? @constant.builtin
    "ARGF" "ARGV" "ENV" "RUBY_ENGINE" "RUBY_PLATFORM" "RUBY_RELEASE_DATE"
    "RUBY_VERSION" "STDERR" "STDIN" "STDOUT" "TOPLEVEL_BINDING"))

(class
  name: (constant) @type)

(module
  name: (constant) @type)

(superclass
  (constant) @type)

(method_parameters
  (identifier) @variable.parameter)

(lambda_parameters
  (identifier) @variable.parameter)

(block_parameters
  (identifier) @variable.parameter)

(splat_parameter
  (identifier) @variable.parameter)

(hash_splat_parameter
  (identifier) @variable.parameter)

(optional_parameter
  name: (identifier) @variable.parameter)

(destructured_parameter
  (identifier) @variable.parameter)

(block_parameter
  (identifier) @variable.parameter)

(keyword_parameter
  name: (identifier) @variable.parameter)

[
  (forward_parameter)
  (forward_argument)
] @variable.parameter

(forward_parameter
  "..." @variable.parameter)

(forward_argument
  "..." @variable.parameter)

(alias
  (identifier) @function)

(setter
  (identifier) @function)

(method
  name: (identifier) @function)

(method
  name: (constant) @type)

(singleton_method
  name: (identifier) @function)

(singleton_method
  name: (constant) @type)

(call
  method: [
    (identifier)
    (constant)
  ] @function.call)

((call
  method: (identifier) @keyword.import)
  (#any-of? @keyword.import "autoload" "load" "require" "require_relative"))

((identifier) @function.builtin
  (#any-of? @function.builtin
    "attr" "attr_accessor" "attr_reader" "attr_writer" "module_function"))

((call
  !receiver
  method: (identifier) @function.builtin)
  (#any-of? @function.builtin "extend" "include" "prepend" "refine" "using"))

((identifier) @keyword.exception
  (#any-of? @keyword.exception "catch" "fail" "raise" "throw"))

((identifier) @keyword.modifier
  (#any-of? @keyword.modifier "private" "protected" "public"))
