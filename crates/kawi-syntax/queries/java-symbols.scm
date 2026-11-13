; Qualified names nest one level per segment, so each depth is spelled out.
(package_declaration
  (identifier) @module)

(package_declaration
  (scoped_identifier
    (identifier) @module))

(package_declaration
  (scoped_identifier
    (scoped_identifier
      (identifier) @module)))

(package_declaration
  (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (identifier) @module))))

(package_declaration
  (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (identifier) @module)))))

(package_declaration
  (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (scoped_identifier
            (identifier) @module))))))

(import_declaration
  (identifier) @module)

(import_declaration
  (scoped_identifier
    (identifier) @module))

(import_declaration
  (scoped_identifier
    (scoped_identifier
      (identifier) @module)))

(import_declaration
  (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (identifier) @module))))

(import_declaration
  (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (identifier) @module)))))

(import_declaration
  (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (scoped_identifier
            (identifier) @module))))))

(module_declaration
  name: (identifier) @module)

(module_declaration
  name: (scoped_identifier
    (identifier) @module))

(module_declaration
  name: (scoped_identifier
    (scoped_identifier
      (identifier) @module)))

(module_declaration
  name: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (identifier) @module))))

(module_declaration
  name: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (identifier) @module)))))

(module_declaration
  name: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (scoped_identifier
            (identifier) @module))))))

(requires_module_directive
  module: (identifier) @module)

(requires_module_directive
  module: (scoped_identifier
    (identifier) @module))

(requires_module_directive
  module: (scoped_identifier
    (scoped_identifier
      (identifier) @module)))

(requires_module_directive
  module: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (identifier) @module))))

(requires_module_directive
  module: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (identifier) @module)))))

(requires_module_directive
  module: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (scoped_identifier
            (identifier) @module))))))

(exports_module_directive
  package: (identifier) @module)

(exports_module_directive
  package: (scoped_identifier
    (identifier) @module))

(exports_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (identifier) @module)))

(exports_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (identifier) @module))))

(exports_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (identifier) @module)))))

(exports_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (scoped_identifier
            (identifier) @module))))))

(opens_module_directive
  package: (identifier) @module)

(opens_module_directive
  package: (scoped_identifier
    (identifier) @module))

(opens_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (identifier) @module)))

(opens_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (identifier) @module))))

(opens_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (identifier) @module)))))

(opens_module_directive
  package: (scoped_identifier
    (scoped_identifier
      (scoped_identifier
        (scoped_identifier
          (scoped_identifier
            (identifier) @module))))))
