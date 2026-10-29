(component
  (type $inner (;0;)
    (instance)
  )
  (type $outer (;1;)
    (instance
      (alias outer 1 $inner (type (;0;)))
      (export (;0;) "editor-read" (instance (type 0)))
    )
  )
  (import "alias" (instance (;0;) (type $outer)))
)
