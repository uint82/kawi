(component
  (type $ty-kawi:plugin/types@0.1.0 (;0;)
    (instance
      (type (;0;) (record (field "line" u32) (field "byte-column" u32)))
      (export (;1;) "position" (type (eq 0)))
      (type (;2;) (enum "error" "warning" "information" "hint"))
      (export (;3;) "severity" (type (eq 2)))
      (type (;4;) (option string))
      (type (;5;) (record (field "start" 1) (field "end" 1) (field "severity" 3) (field "message" string) (field "source" 4) (field "code" 4)))
      (export (;6;) "diagnostic" (type (eq 5)))
      (type (;7;) (record (field "code" s32) (field "message" string) (field "capability" 4)))
      (export (;8;) "error" (type (eq 7)))
      (type (;9;) (variant (case "null") (case "boolean" bool) (case "integer" s64) (case "float" f64) (case "text" string)))
      (export (;10;) "argument" (type (eq 9)))
      (type (;11;) (record (field "start" 1) (field "end" 1) (field "text" string)))
      (export (;12;) "edit" (type (eq 11)))
      (type (;13;) (list 12))
      (type (;14;) (record (field "buffer" u64) (field "revision" u64) (field "previous-revision" u64) (field "origin" string) (field "full-resync" bool) (field "changes" 13)))
      (export (;15;) "buffer-changed" (type (eq 14)))
      (type (;16;) (record (field "buffer" u64) (field "path" 4) (field "language" 4) (field "revision" u64)))
      (export (;17;) "buffer-opened" (type (eq 16)))
      (type (;18;) (record (field "buffer" u64) (field "path" string) (field "language" 4) (field "revision" u64)))
      (export (;19;) "file-opened" (type (eq 18)))
      (type (;20;) (record (field "buffer" u64) (field "path" 4)))
      (export (;21;) "buffer-closed" (type (eq 20)))
      (type (;22;) (record (field "buffer" u64) (field "path" 4) (field "revision" u64)))
      (export (;23;) "file-saved" (type (eq 22)))
      (type (;24;) (record (field "buffer" u64) (field "previous" string) (field "next" string)))
      (export (;25;) "mode-changed" (type (eq 24)))
      (type (;26;) (option u64))
      (type (;27;) (list 6))
      (type (;28;) (record (field "buffer" 26) (field "path" string) (field "revision" u64) (field "diagnostics" 27)))
      (export (;29;) "diagnostics-changed" (type (eq 28)))
      (type (;30;) (variant (case "buffer-changed" 15) (case "buffer-opened" 17) (case "file-opened" 19) (case "buffer-closed" 21) (case "file-saved" 23) (case "mode-changed" 25) (case "diagnostics-changed" 29)))
      (export (;31;) "event" (type (eq 30)))
    )
  )
  (import "kawi:plugin/types@0.1.0" (instance $kawi:plugin/types@0.1.0 (;0;) (type $ty-kawi:plugin/types@0.1.0)))
  (alias export $kawi:plugin/types@0.1.0 "diagnostic" (type $diagnostic (;1;)))
  (alias export $kawi:plugin/types@0.1.0 "error" (type $error (;2;)))
  (type $ty-kawi:plugin/diagnostics-publish@0.1.0 (;3;)
    (instance
      (alias outer 1 $diagnostic (type (;0;)))
      (export (;1;) "diagnostic" (type (eq 0)))
      (alias outer 1 $error (type (;2;)))
      (export (;3;) "error" (type (eq 2)))
      (type (;4;) (list 1))
      (type (;5;) (result (error 3)))
      (type (;6;) (func (param "source" string) (param "buffer" u64) (param "revision" u64) (param "items" 4) (result 5)))
      (export (;0;) "set" (func (type 6)))
    )
  )
  (import "kawi:plugin/diagnostics-publish@0.1.0" (instance $kawi:plugin/diagnostics-publish@0.1.0 (;1;) (type $ty-kawi:plugin/diagnostics-publish@0.1.0)))
  (core module $main (;0;)
    (type (;0;) (func (param i32 i32 i64 i64 i32 i32 i32)))
    (type (;1;) (func (param i32 i32 i32 i32) (result i32)))
    (type (;2;) (func (param i32) (result i32)))
    (type (;3;) (func (param i32 i64 i64 i64 i32 i64 i32 i64 i64)))
    (import "kawi:plugin/diagnostics-publish@0.1.0" "set" (func $set (;0;) (type 0) (param i32 i32 i64 i64 i32 i32 i32)))
    (memory (;0;) 34)
    (global $heap (;0;) (mut i32) i32.const 2097152)
    (export "memory" (memory 0))
    (export "cabi_realloc" (func 1))
    (export "kawi:plugin/lifecycle@0.1.0#init" (func 2))
    (export "kawi:plugin/lifecycle@0.1.0#invoke-command" (func 3))
    (export "kawi:plugin/lifecycle@0.1.0#on-event" (func 4))
    (func (;1;) (type 1) (param i32 i32 i32 i32) (result i32)
      (local $ptr i32)
      global.get $heap
      local.get 2
      i32.const 1
      i32.sub
      i32.add
      i32.const 0
      local.get 2
      i32.sub
      i32.and
      local.tee $ptr
      local.get 3
      i32.add
      global.set $heap
      local.get $ptr
    )
    (func (;2;) (type 2) (param i32) (result i32)
      i32.const 16
      i32.const 0
      i32.store8
      i32.const 16
    )
    (func (;3;) (type 1) (param i32 i32 i32 i32) (result i32)
      local.get 1
      i32.eqz
      if ;; label = @1
        i32.const 32
        i32.const 0
        i32.store8
        i32.const 40
        i32.const 4
        i32.store8
        i32.const 48
        i32.const 128
        i32.store
        i32.const 52
        i32.const 4
        i32.store
        i32.const 32
        return
      end
      local.get 0
      i32.load8_u
      i32.const 100
      i32.eq
      if ;; label = @1
        i32.const 0
        i32.const 0
        i64.const 1
        i64.const 7
        i32.const 4096
        i32.const 1001
        i32.const 2048
        call $set
      i32.const 2048
      i32.load8_u
      if
        i32.const 32
        i32.const 1
        i32.store8
        i32.const 40
        i32.const -32602
        i32.store
        i32.const 44
        i32.const 128
        i32.store
        i32.const 48
        i32.const 4
        i32.store
        i32.const 52
        i32.const 0
        i32.store8
        i32.const 32
        return
      end
      end
      i32.const 32
      i32.const 0
      i32.store8
      i32.const 40
      i32.const 4
      i32.store8
      i32.const 48
      i32.const 128
      i32.store
      i32.const 52
      i32.const 4
      i32.store
      i32.const 32
    )
    (func (;4;) (type 3) (param i32 i64 i64 i64 i32 i64 i32 i64 i64))
    (data (;0;) (i32.const 128) "pong")
    (@producers
      (processed-by "wit-component" "0.262.0")
    )
  )
  (core module $wit-component-shim-module (;1;)
    (type (;0;) (func (param i32 i32 i64 i64 i32 i32 i32)))
    (table (;0;) 1 1 funcref)
    (export "0" (func $indirect-kawi:plugin/diagnostics-publish@0.1.0-set))
    (export "$imports" (table 0))
    (func $indirect-kawi:plugin/diagnostics-publish@0.1.0-set (;0;) (type 0) (param i32 i32 i64 i64 i32 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      local.get 5
      local.get 6
      i32.const 0
      call_indirect (type 0)
    )
    (@producers
      (processed-by "wit-component" "0.262.0")
    )
  )
  (core instance $wit-component-shim-instance (;0;) (instantiate $wit-component-shim-module))
  (alias export $kawi:plugin/types@0.1.0 "argument" (type $argument (;4;)))
  (alias export $kawi:plugin/types@0.1.0 "error" (type $"#type5 error" (@name "error") (;5;)))
  (alias export $kawi:plugin/types@0.1.0 "event" (type $event (;6;)))
  (alias core export $wit-component-shim-instance "0" (core func $indirect-kawi:plugin/diagnostics-publish@0.1.0-set (;0;)))
  (core instance $kawi:plugin/diagnostics-publish@0.1.0 (;1;)
    (export "set" (func $indirect-kawi:plugin/diagnostics-publish@0.1.0-set))
  )
  (core instance $main (;2;) (instantiate $main
      (with "kawi:plugin/diagnostics-publish@0.1.0" (instance $kawi:plugin/diagnostics-publish@0.1.0))
    )
  )
  (alias core export $main "memory" (core memory $memory (;0;)))
  (core module $wit-component-fixup (;2;)
    (type (;0;) (func (param i32 i32 i64 i64 i32 i32 i32)))
    (import "actual" "0" (func $0 (;0;) (type 0) (param i32 i32 i64 i64 i32 i32 i32)))
    (import "shim" "$imports" (table (;0;) 1 1 funcref))
    (elem (;0;) (i32.const 0) func $0)
    (@producers
      (processed-by "wit-component" "0.262.0")
    )
  )
  (alias export $kawi:plugin/diagnostics-publish@0.1.0 "set" (func $set (;0;)))
  (alias core export $main "cabi_realloc" (core func $cabi_realloc (;1;)))
  (core func $"#core-func2 indirect-kawi:plugin/diagnostics-publish@0.1.0-set" (@name "indirect-kawi:plugin/diagnostics-publish@0.1.0-set") (;2;) (canon lower (func $set) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (core instance $actual (;3;)
    (export "0" (func $"#core-func2 indirect-kawi:plugin/diagnostics-publish@0.1.0-set"))
  )
  (core instance $fixup (;4;) (instantiate $wit-component-fixup
      (with "actual" (instance $actual))
      (with "shim" (instance $wit-component-shim-instance))
    )
  )
  (type (;7;) (result (error $"#type5 error")))
  (type (;8;) (func (param "host-protocol" u32) (result 7)))
  (alias core export $main "kawi:plugin/lifecycle@0.1.0#init" (core func $kawi:plugin/lifecycle@0.1.0#init (;3;)))
  (func $init (;1;) (type 8) (canon lift (core func $kawi:plugin/lifecycle@0.1.0#init) (memory $memory) string-encoding=utf8))
  (type (;9;) (list $argument))
  (type (;10;) (result $argument (error $"#type5 error")))
  (type (;11;) (func (param "id" string) (param "args" 9) (result 10)))
  (alias core export $main "kawi:plugin/lifecycle@0.1.0#invoke-command" (core func $kawi:plugin/lifecycle@0.1.0#invoke-command (;4;)))
  (func $invoke-command (;2;) (type 11) (canon lift (core func $kawi:plugin/lifecycle@0.1.0#invoke-command) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (type (;12;) (func (param "event" $event)))
  (alias core export $main "kawi:plugin/lifecycle@0.1.0#on-event" (core func $kawi:plugin/lifecycle@0.1.0#on-event (;5;)))
  (func $on-event (;3;) (type 12) (canon lift (core func $kawi:plugin/lifecycle@0.1.0#on-event) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (alias export $kawi:plugin/types@0.1.0 "position" (type $position (;13;)))
  (alias export $kawi:plugin/types@0.1.0 "edit" (type $edit (;14;)))
  (alias export $kawi:plugin/types@0.1.0 "buffer-changed" (type $buffer-changed (;15;)))
  (alias export $kawi:plugin/types@0.1.0 "buffer-opened" (type $buffer-opened (;16;)))
  (alias export $kawi:plugin/types@0.1.0 "file-opened" (type $file-opened (;17;)))
  (alias export $kawi:plugin/types@0.1.0 "buffer-closed" (type $buffer-closed (;18;)))
  (alias export $kawi:plugin/types@0.1.0 "file-saved" (type $file-saved (;19;)))
  (alias export $kawi:plugin/types@0.1.0 "mode-changed" (type $mode-changed (;20;)))
  (alias export $kawi:plugin/types@0.1.0 "severity" (type $severity (;21;)))
  (alias export $kawi:plugin/types@0.1.0 "diagnostic" (type $"#type22 diagnostic" (@name "diagnostic") (;22;)))
  (alias export $kawi:plugin/types@0.1.0 "diagnostics-changed" (type $diagnostics-changed (;23;)))
  (component $kawi:plugin/lifecycle@0.1.0-shim-component (;0;)
    (type (;0;) (variant (case "null") (case "boolean" bool) (case "integer" s64) (case "float" f64) (case "text" string)))
    (import "import-type-argument" (type (;1;) (eq 0)))
    (type (;2;) (option string))
    (type (;3;) (record (field "code" s32) (field "message" string) (field "capability" 2)))
    (import "import-type-error" (type (;4;) (eq 3)))
    (type (;5;) (record (field "line" u32) (field "byte-column" u32)))
    (import "import-type-position" (type (;6;) (eq 5)))
    (type (;7;) (record (field "start" 6) (field "end" 6) (field "text" string)))
    (import "import-type-edit" (type (;8;) (eq 7)))
    (type (;9;) (list 8))
    (type (;10;) (record (field "buffer" u64) (field "revision" u64) (field "previous-revision" u64) (field "origin" string) (field "full-resync" bool) (field "changes" 9)))
    (import "import-type-buffer-changed" (type (;11;) (eq 10)))
    (type (;12;) (record (field "buffer" u64) (field "path" 2) (field "language" 2) (field "revision" u64)))
    (import "import-type-buffer-opened" (type (;13;) (eq 12)))
    (type (;14;) (record (field "buffer" u64) (field "path" string) (field "language" 2) (field "revision" u64)))
    (import "import-type-file-opened" (type (;15;) (eq 14)))
    (type (;16;) (record (field "buffer" u64) (field "path" 2)))
    (import "import-type-buffer-closed" (type (;17;) (eq 16)))
    (type (;18;) (record (field "buffer" u64) (field "path" 2) (field "revision" u64)))
    (import "import-type-file-saved" (type (;19;) (eq 18)))
    (type (;20;) (record (field "buffer" u64) (field "previous" string) (field "next" string)))
    (import "import-type-mode-changed" (type (;21;) (eq 20)))
    (type (;22;) (enum "error" "warning" "information" "hint"))
    (import "import-type-severity" (type (;23;) (eq 22)))
    (type (;24;) (record (field "start" 6) (field "end" 6) (field "severity" 23) (field "message" string) (field "source" 2) (field "code" 2)))
    (import "import-type-diagnostic" (type (;25;) (eq 24)))
    (type (;26;) (option u64))
    (type (;27;) (list 25))
    (type (;28;) (record (field "buffer" 26) (field "path" string) (field "revision" u64) (field "diagnostics" 27)))
    (import "import-type-diagnostics-changed" (type (;29;) (eq 28)))
    (type (;30;) (variant (case "buffer-changed" 11) (case "buffer-opened" 13) (case "file-opened" 15) (case "buffer-closed" 17) (case "file-saved" 19) (case "mode-changed" 21) (case "diagnostics-changed" 29)))
    (import "import-type-event" (type (;31;) (eq 30)))
    (import "import-type-error0" (type (;32;) (eq 4)))
    (type (;33;) (result (error 32)))
    (type (;34;) (func (param "host-protocol" u32) (result 33)))
    (import "import-func-init" (func (;0;) (type 34)))
    (import "import-type-argument0" (type (;35;) (eq 1)))
    (type (;36;) (list 35))
    (type (;37;) (result 35 (error 32)))
    (type (;38;) (func (param "id" string) (param "args" 36) (result 37)))
    (import "import-func-invoke-command" (func (;1;) (type 38)))
    (import "import-type-event0" (type (;39;) (eq 31)))
    (type (;40;) (func (param "event" 39)))
    (import "import-func-on-event" (func (;2;) (type 40)))
    (export (;41;) "argument" (type 1))
    (export (;42;) "error" (type 4))
    (export (;43;) "event" (type 31))
    (type (;44;) (result (error 42)))
    (type (;45;) (func (param "host-protocol" u32) (result 44)))
    (export (;3;) "init" (func 0) (func (type 45)))
    (type (;46;) (list 41))
    (type (;47;) (result 41 (error 42)))
    (type (;48;) (func (param "id" string) (param "args" 46) (result 47)))
    (export (;4;) "invoke-command" (func 1) (func (type 48)))
    (type (;49;) (func (param "event" 43)))
    (export (;5;) "on-event" (func 2) (func (type 49)))
  )
  (instance $kawi:plugin/lifecycle@0.1.0-shim-instance (;2;) (instantiate $kawi:plugin/lifecycle@0.1.0-shim-component
      (with "import-func-init" (func $init))
      (with "import-func-invoke-command" (func $invoke-command))
      (with "import-func-on-event" (func $on-event))
      (with "import-type-argument" (type $argument))
      (with "import-type-error" (type $"#type5 error"))
      (with "import-type-position" (type $position))
      (with "import-type-edit" (type $edit))
      (with "import-type-buffer-changed" (type $buffer-changed))
      (with "import-type-buffer-opened" (type $buffer-opened))
      (with "import-type-file-opened" (type $file-opened))
      (with "import-type-buffer-closed" (type $buffer-closed))
      (with "import-type-file-saved" (type $file-saved))
      (with "import-type-mode-changed" (type $mode-changed))
      (with "import-type-severity" (type $severity))
      (with "import-type-diagnostic" (type $"#type22 diagnostic"))
      (with "import-type-diagnostics-changed" (type $diagnostics-changed))
      (with "import-type-event" (type $event))
      (with "import-type-error0" (type $"#type5 error"))
      (with "import-type-argument0" (type $argument))
      (with "import-type-event0" (type $event))
    )
  )
  (export $kawi:plugin/lifecycle@0.1.0 (;3;) "kawi:plugin/lifecycle@0.1.0" (instance $kawi:plugin/lifecycle@0.1.0-shim-instance))
  (@producers
    (processed-by "wit-component" "0.262.0")
  )
)
