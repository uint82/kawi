; The stock query leaves attribute paths and nested subscripts in annotations
; as variables.
(type
  (attribute
    (identifier) @type))

(type
  (subscript value:
    (attribute
      (identifier) @type)))

(type
  (subscript subscript:
    (attribute
      (identifier) @type)))

(type
  (subscript subscript: (list
    (attribute
      (identifier) @type))))

(type
  (subscript subscript: (subscript value:
    (attribute
      (identifier) @type))))

(type
  (subscript subscript: (subscript subscript:
    (attribute
      (identifier) @type))))

(type
  (binary_operator left:
    (attribute
      (identifier) @type)))

(type
  (binary_operator right:
    (attribute
      (identifier) @type)))
