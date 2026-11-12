; Qualified names nest one level per segment, so each depth is spelled out.
(using_directive
  (identifier) @module)

(using_directive
  (qualified_name (identifier) @module))

(using_directive
  (qualified_name (qualified_name (identifier) @module)))

(using_directive
  (qualified_name (qualified_name (qualified_name (identifier) @module))))

(using_directive
  (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))

(using_directive
  (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module))))))

(using_directive
  (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))))

(using_directive
  (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module))))))))

(using_directive
  (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))))))

(namespace_declaration
  name: (qualified_name (identifier) @module))

(namespace_declaration
  name: (qualified_name (qualified_name (identifier) @module)))

(namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (identifier) @module))))

(namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))

(namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module))))))

(namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))))

(namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module))))))))

(namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))))))

(file_scoped_namespace_declaration
  name: (qualified_name (identifier) @module))

(file_scoped_namespace_declaration
  name: (qualified_name (qualified_name (identifier) @module)))

(file_scoped_namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (identifier) @module))))

(file_scoped_namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))

(file_scoped_namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module))))))

(file_scoped_namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))))

(file_scoped_namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module))))))))

(file_scoped_namespace_declaration
  name: (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (qualified_name (identifier) @module)))))))))

(using_directive
  name: (identifier) @type)

(enum_member_declaration
  name: (identifier) @constant)
