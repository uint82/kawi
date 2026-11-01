(component
  (type $ty-kawi:plugin/types@0.5.0 (;0;)
    (instance
      (type (;0;) (record (field "line" u32) (field "byte-column" u32)))
      (export (;1;) "position" (type (eq 0)))
      (type (;2;) (record (field "start" 1) (field "end" 1)))
      (export (;3;) "selection" (type (eq 2)))
      (type (;4;) (option string))
      (type (;5;) (option 3))
      (type (;6;) (record (field "buffer" u64) (field "revision" u64) (field "path" 4) (field "language" 4) (field "mode" string) (field "cursor" 1) (field "selection" 5) (field "text" 4)))
      (export (;7;) "state" (type (eq 6)))
      (type (;8;) (record (field "code" s32) (field "message" string) (field "capability" 4)))
      (export (;9;) "error" (type (eq 8)))
      (type (;10;) (enum "error" "warning" "information" "hint"))
      (export (;11;) "severity" (type (eq 10)))
      (type (;12;) (record (field "start" 1) (field "end" 1) (field "severity" 11) (field "message" string) (field "source" 4) (field "code" 4)))
      (export (;13;) "diagnostic" (type (eq 12)))
      (type (;14;) (variant (case "null") (case "boolean" bool) (case "integer" s64) (case "float" f64) (case "text" string)))
      (export (;15;) "argument" (type (eq 14)))
      (type (;16;) (record (field "start" 1) (field "end" 1) (field "text" string)))
      (export (;17;) "edit" (type (eq 16)))
      (type (;18;) (list 17))
      (type (;19;) (record (field "buffer" u64) (field "revision" u64) (field "previous-revision" u64) (field "origin" string) (field "full-resync" bool) (field "changes" 18)))
      (export (;20;) "buffer-changed" (type (eq 19)))
      (type (;21;) (record (field "buffer" u64) (field "path" 4) (field "language" 4) (field "revision" u64)))
      (export (;22;) "buffer-opened" (type (eq 21)))
      (type (;23;) (record (field "buffer" u64) (field "path" string) (field "language" 4) (field "revision" u64)))
      (export (;24;) "file-opened" (type (eq 23)))
      (type (;25;) (record (field "buffer" u64) (field "path" 4)))
      (export (;26;) "buffer-closed" (type (eq 25)))
      (type (;27;) (record (field "buffer" u64) (field "path" 4) (field "revision" u64)))
      (export (;28;) "file-saved" (type (eq 27)))
      (type (;29;) (record (field "buffer" u64) (field "previous" string) (field "next" string)))
      (export (;30;) "mode-changed" (type (eq 29)))
      (type (;31;) (option u64))
      (type (;32;) (list 13))
      (type (;33;) (record (field "buffer" 31) (field "path" string) (field "revision" u64) (field "diagnostics" 32)))
      (export (;34;) "diagnostics-changed" (type (eq 33)))
      (type (;35;) (variant (case "buffer-changed" 20) (case "buffer-opened" 22) (case "file-opened" 24) (case "buffer-closed" 26) (case "file-saved" 28) (case "mode-changed" 30) (case "diagnostics-changed" 34)))
      (export (;36;) "event" (type (eq 35)))
    )
  )
  (import "kawi:plugin/types@0.5.0" (instance $kawi:plugin/types@0.5.0 (;0;) (type $ty-kawi:plugin/types@0.5.0)))
  (alias export $kawi:plugin/types@0.5.0 "state" (type $state (;1;)))
  (alias export $kawi:plugin/types@0.5.0 "error" (type $error (;2;)))
  (type $ty-kawi:plugin/editor-read@0.5.0 (;3;)
    (instance
      (alias outer 1 $state (type (;0;)))
      (export (;1;) "state" (type (eq 0)))
      (alias outer 1 $error (type (;2;)))
      (export (;3;) "error" (type (eq 2)))
      (type (;4;) (result 1 (error 3)))
      (type (;5;) (func (param "include-text" bool) (result 4)))
      (export (;0;) "get-state" (func (type 5)))
    )
  )
  (import "kawi:plugin/editor-read@0.5.0" (instance $kawi:plugin/editor-read@0.5.0 (;1;) (type $ty-kawi:plugin/editor-read@0.5.0)))
  (alias export $kawi:plugin/types@0.5.0 "error" (type $"#type4 error" (@name "error") (;4;)))
  (type $ty-kawi:plugin/ui-message@0.5.0 (;5;)
    (instance
      (alias outer 1 $"#type4 error" (type (;0;)))
      (export (;1;) "error" (type (eq 0)))
      (type (;2;) (result (error 1)))
      (type (;3;) (func (param "text" string) (result 2)))
      (export (;0;) "message" (func (type 3)))
    )
  )
  (import "kawi:plugin/ui-message@0.5.0" (instance $kawi:plugin/ui-message@0.5.0 (;2;) (type $ty-kawi:plugin/ui-message@0.5.0)))
  (alias export $kawi:plugin/types@0.5.0 "diagnostic" (type $diagnostic (;6;)))
  (alias export $kawi:plugin/types@0.5.0 "error" (type $"#type7 error" (@name "error") (;7;)))
  (type $ty-kawi:plugin/diagnostics-publish@0.5.0 (;8;)
    (instance
      (alias outer 1 $diagnostic (type (;0;)))
      (export (;1;) "diagnostic" (type (eq 0)))
      (alias outer 1 $"#type7 error" (type (;2;)))
      (export (;3;) "error" (type (eq 2)))
      (type (;4;) (list 1))
      (type (;5;) (result (error 3)))
      (type (;6;) (func (param "source" string) (param "buffer" u64) (param "revision" u64) (param "items" 4) (result 5)))
      (export (;0;) "set" (func (type 6)))
    )
  )
  (import "kawi:plugin/diagnostics-publish@0.5.0" (instance $kawi:plugin/diagnostics-publish@0.5.0 (;3;) (type $ty-kawi:plugin/diagnostics-publish@0.5.0)))
  (core module $main (;0;)
    (type (;0;) (func (param i32 i32)))
    (type (;1;) (func (param i32 i32 i32)))
    (type (;2;) (func (param i32 i32 i64 i64 i32 i32 i32)))
    (type (;3;) (func (param i32)))
    (type (;4;) (func (param i32 i32 i32 i32) (result i32)))
    (type (;5;) (func (param i32 i32 i32) (result i32)))
    (type (;6;) (func (param i32 i32) (result i32)))
    (type (;7;) (func (param i32 i64 i64 i32 i32) (result i32)))
    (type (;8;) (func (param i32) (result i32)))
    (type (;9;) (func (param i32 i64 i64 i64 i32 i64 i32 i64 i64)))
    (type (;10;) (func))
    (import "kawi:plugin/editor-read@0.5.0" "get-state" (func $__wasm_import_kawi_plugin_editor_read_get_state (;0;) (type 0) (param i32 i32)))
    (import "kawi:plugin/ui-message@0.5.0" "message" (func $__wasm_import_kawi_plugin_ui_message_message (;1;) (type 1) (param i32 i32 i32)))
    (import "kawi:plugin/diagnostics-publish@0.5.0" "set" (func $__wasm_import_kawi_plugin_diagnostics_publish_set (;2;) (type 2) (param i32 i32 i64 i64 i32 i32 i32)))
    (table (;0;) 1 1 funcref)
    (memory (;0;) 3)
    (global $__stack_pointer (;0;) (mut i32) i32.const 65536)
    (export "memory" (memory 0))
    (export "__stack_pointer" (global $__stack_pointer))
    (export "cabi_post_kawi:plugin/lifecycle@0.5.0#init" (func $__wasm_export_exports_kawi_plugin_lifecycle_init_post_return))
    (export "cabi_post_kawi:plugin/lifecycle@0.5.0#invoke-command" (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command_post_return))
    (export "cabi_realloc" (func $cabi_realloc))
    (export "kawi:plugin/lifecycle@0.5.0#init" (func $__wasm_export_exports_kawi_plugin_lifecycle_init))
    (export "kawi:plugin/lifecycle@0.5.0#invoke-command" (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command))
    (export "kawi:plugin/lifecycle@0.5.0#on-event" (func $__wasm_export_exports_kawi_plugin_lifecycle_on_event))
    (export "__component_type_object_force_link_host" (func $__component_type_object_force_link_host))
    (func $__wasm_export_exports_kawi_plugin_lifecycle_init_post_return (;3;) (type 3) (param i32)
      block ;; label = @1
        local.get 0
        i32.load8_u
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.load offset=12
          i32.eqz
          br_if 0 (;@2;)
          local.get 0
          i32.load offset=8
          call $free
        end
        local.get 0
        i32.load8_u offset=16
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        local.get 0
        i32.load offset=24
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        i32.load offset=20
        call $free
      end
    )
    (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command_post_return (;4;) (type 3) (param i32)
      (local i32)
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              local.get 0
              i32.load8_u
              br_table 0 (;@4;) 1 (;@3;) 3 (;@1;)
            end
            local.get 0
            i32.load8_u offset=8
            i32.const 4
            i32.ne
            br_if 2 (;@1;)
            local.get 0
            i32.load offset=20
            i32.eqz
            br_if 2 (;@1;)
            i32.const 16
            local.set 1
            br 1 (;@2;)
          end
          block ;; label = @3
            local.get 0
            i32.load offset=16
            i32.eqz
            br_if 0 (;@3;)
            local.get 0
            i32.load offset=12
            call $free
          end
          local.get 0
          i32.load8_u offset=20
          i32.const 1
          i32.ne
          br_if 1 (;@1;)
          local.get 0
          i32.load offset=28
          i32.eqz
          br_if 1 (;@1;)
          i32.const 24
          local.set 1
        end
        local.get 0
        local.get 1
        i32.add
        i32.load
        call $free
      end
    )
    (func $cabi_realloc (;5;) (type 4) (param i32 i32 i32 i32) (result i32)
      block ;; label = @1
        local.get 3
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        local.get 3
        call $realloc
        local.tee 2
        br_if 0 (;@1;)
        call $abort
        unreachable
      end
      local.get 2
    )
    (func $kawi_plugin_types_state_free (;6;) (type 3) (param i32)
      block ;; label = @1
        local.get 0
        i32.load8_u offset=16
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.load offset=24
          i32.eqz
          br_if 0 (;@2;)
          local.get 0
          i32.load offset=20
          call $free
        end
        local.get 0
        i64.const 0
        i64.store offset=20 align=4
      end
      block ;; label = @1
        local.get 0
        i32.load8_u offset=28
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.load offset=36
          i32.eqz
          br_if 0 (;@2;)
          local.get 0
          i32.load offset=32
          call $free
        end
        local.get 0
        i64.const 0
        i64.store offset=32 align=4
      end
      block ;; label = @1
        local.get 0
        i32.load offset=44
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        i32.load offset=40
        call $free
      end
      local.get 0
      i64.const 0
      i64.store offset=40 align=4
      block ;; label = @1
        local.get 0
        i32.load8_u offset=76
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.load offset=84
          i32.eqz
          br_if 0 (;@2;)
          local.get 0
          i32.load offset=80
          call $free
        end
        local.get 0
        i64.const 0
        i64.store offset=80 align=4
      end
    )
    (func $kawi_plugin_editor_read_state_free (;7;) (type 3) (param i32)
      local.get 0
      call $kawi_plugin_types_state_free
    )
    (func $kawi_plugin_editor_read_error_free (;8;) (type 3) (param i32)
      block ;; label = @1
        local.get 0
        i32.load offset=8
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        i32.load offset=4
        call $free
      end
      local.get 0
      i64.const 0
      i64.store offset=4 align=4
      block ;; label = @1
        local.get 0
        i32.load8_u offset=12
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.load offset=20
          i32.eqz
          br_if 0 (;@2;)
          local.get 0
          i32.load offset=16
          call $free
        end
        local.get 0
        i64.const 0
        i64.store offset=16 align=4
      end
    )
    (func $kawi_plugin_ui_message_error_free (;9;) (type 3) (param i32)
      block ;; label = @1
        local.get 0
        i32.load offset=8
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        i32.load offset=4
        call $free
      end
      local.get 0
      i64.const 0
      i64.store offset=4 align=4
      block ;; label = @1
        local.get 0
        i32.load8_u offset=12
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.load offset=20
          i32.eqz
          br_if 0 (;@2;)
          local.get 0
          i32.load offset=16
          call $free
        end
        local.get 0
        i64.const 0
        i64.store offset=16 align=4
      end
    )
    (func $kawi_plugin_diagnostics_publish_error_free (;10;) (type 3) (param i32)
      block ;; label = @1
        local.get 0
        i32.load offset=8
        i32.eqz
        br_if 0 (;@1;)
        local.get 0
        i32.load offset=4
        call $free
      end
      local.get 0
      i64.const 0
      i64.store offset=4 align=4
      block ;; label = @1
        local.get 0
        i32.load8_u offset=12
        i32.const 1
        i32.ne
        br_if 0 (;@1;)
        block ;; label = @2
          local.get 0
          i32.load offset=20
          i32.eqz
          br_if 0 (;@2;)
          local.get 0
          i32.load offset=16
          call $free
        end
        local.get 0
        i64.const 0
        i64.store offset=16 align=4
      end
    )
    (func $kawi_plugin_editor_read_get_state (;11;) (type 5) (param i32 i32 i32) (result i32)
      (local i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
      global.get $__stack_pointer
      i32.const 96
      i32.sub
      local.tee 3
      global.set $__stack_pointer
      local.get 0
      local.get 3
      call $__wasm_import_kawi_plugin_editor_read_get_state
      i64.const 0
      local.set 4
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              local.get 3
              i32.load8_u
              br_table 0 (;@4;) 2 (;@2;) 1 (;@3;)
            end
            local.get 3
            i32.load8_u offset=36
            local.set 2
            local.get 3
            i32.load8_u offset=24
            local.set 0
            block ;; label = @4
              block ;; label = @5
                local.get 3
                i32.load8_u offset=64
                i32.const 1
                i32.eq
                br_if 0 (;@5;)
                i32.const 0
                local.set 5
                br 1 (;@4;)
              end
              local.get 3
              i32.load offset=80
              local.set 6
              local.get 3
              i32.load offset=76
              local.set 7
              local.get 3
              i32.load offset=72
              local.set 8
              local.get 3
              i32.load offset=68
              local.set 9
              i32.const 1
              local.set 5
            end
            local.get 2
            i32.const 1
            i32.eq
            local.set 10
            local.get 0
            i32.const 1
            i32.eq
            local.set 11
            local.get 3
            i32.load offset=44
            local.set 12
            local.get 3
            i32.load offset=40
            local.set 13
            local.get 3
            i32.load offset=32
            local.set 14
            local.get 3
            i32.load offset=28
            local.set 15
            local.get 3
            i32.load8_u offset=84
            i32.const 1
            i32.eq
            local.set 16
            local.get 3
            i64.load offset=8
            local.tee 4
            i64.const 32
            i64.shr_u
            i32.wrap_i64
            local.set 17
            local.get 4
            i32.wrap_i64
            local.set 0
            local.get 3
            i32.load offset=88
            local.set 18
            local.get 3
            i32.load offset=92
            local.set 19
            local.get 3
            i32.load offset=60
            local.set 20
            local.get 3
            i32.load offset=56
            local.set 21
            local.get 3
            i32.load offset=52
            local.set 22
            local.get 3
            i32.load offset=48
            local.set 23
            local.get 3
            i64.load offset=16
            local.set 4
          end
          local.get 1
          local.get 19
          i32.store offset=84
          local.get 1
          local.get 18
          i32.store offset=80
          local.get 1
          local.get 16
          i32.store8 offset=76
          local.get 1
          local.get 6
          i32.store offset=72
          local.get 1
          local.get 7
          i32.store offset=68
          local.get 1
          local.get 8
          i32.store offset=64
          local.get 1
          local.get 9
          i32.store offset=60
          local.get 1
          local.get 5
          i32.store8 offset=56
          local.get 1
          local.get 20
          i32.store offset=52
          local.get 1
          local.get 21
          i32.store offset=48
          local.get 1
          local.get 22
          i32.store offset=44
          local.get 1
          local.get 23
          i32.store offset=40
          local.get 1
          local.get 12
          i32.store offset=36
          local.get 1
          local.get 13
          i32.store offset=32
          local.get 1
          local.get 10
          i32.store8 offset=28
          local.get 1
          local.get 14
          i32.store offset=24
          local.get 1
          local.get 15
          i32.store offset=20
          local.get 1
          local.get 11
          i32.store offset=16
          local.get 1
          local.get 4
          i64.store offset=8
          local.get 1
          local.get 17
          i32.store offset=4
          local.get 1
          local.get 0
          i32.store
          i32.const 1
          local.set 1
          br 1 (;@1;)
        end
        local.get 2
        local.get 3
        i64.load offset=8
        i64.store align=4
        local.get 2
        local.get 3
        i32.load offset=28
        i32.store offset=20
        local.get 2
        local.get 3
        i32.load offset=24
        i32.store offset=16
        local.get 2
        i64.const 4294967296
        i64.const 0
        local.get 3
        i32.load8_u offset=20
        i32.const 1
        i32.eq
        select
        local.get 3
        i64.load32_u offset=16
        i64.or
        i64.store offset=8 align=4
        i32.const 0
        local.set 1
      end
      local.get 3
      i32.const 96
      i32.add
      global.set $__stack_pointer
      local.get 1
    )
    (func $kawi_plugin_ui_message_message (;12;) (type 6) (param i32 i32) (result i32)
      (local i32)
      global.get $__stack_pointer
      i32.const 32
      i32.sub
      local.tee 2
      global.set $__stack_pointer
      local.get 0
      i32.load
      local.get 0
      i32.load offset=4
      local.get 2
      i32.const 4
      i32.add
      call $__wasm_import_kawi_plugin_ui_message_message
      block ;; label = @1
        local.get 2
        i32.load8_u offset=4
        i32.const 1
        i32.ne
        local.tee 0
        br_if 0 (;@1;)
        local.get 1
        local.get 2
        i32.load offset=16
        i32.store offset=8
        local.get 1
        local.get 2
        i64.load offset=8 align=4
        i64.store align=4
        local.get 1
        local.get 2
        i32.load offset=28
        i32.store offset=20
        local.get 1
        local.get 2
        i32.load offset=24
        i32.store offset=16
        local.get 1
        local.get 2
        i32.load8_u offset=20
        i32.const 1
        i32.eq
        i32.store8 offset=12
      end
      local.get 2
      i32.const 32
      i32.add
      global.set $__stack_pointer
      local.get 0
    )
    (func $kawi_plugin_diagnostics_publish_set (;13;) (type 7) (param i32 i64 i64 i32 i32) (result i32)
      (local i32)
      global.get $__stack_pointer
      i32.const 32
      i32.sub
      local.tee 5
      global.set $__stack_pointer
      local.get 0
      i32.load
      local.get 0
      i32.load offset=4
      local.get 1
      local.get 2
      local.get 3
      i32.load
      local.get 3
      i32.load offset=4
      local.get 5
      i32.const 4
      i32.add
      call $__wasm_import_kawi_plugin_diagnostics_publish_set
      block ;; label = @1
        local.get 5
        i32.load8_u offset=4
        i32.const 1
        i32.ne
        local.tee 3
        br_if 0 (;@1;)
        local.get 4
        local.get 5
        i32.load offset=16
        i32.store offset=8
        local.get 4
        local.get 5
        i64.load offset=8 align=4
        i64.store align=4
        local.get 4
        local.get 5
        i32.load offset=28
        i32.store offset=20
        local.get 4
        local.get 5
        i32.load offset=24
        i32.store offset=16
        local.get 4
        local.get 5
        i32.load8_u offset=20
        i32.const 1
        i32.eq
        i32.store8 offset=12
      end
      local.get 5
      i32.const 32
      i32.add
      global.set $__stack_pointer
      local.get 3
    )
    (func $__wasm_export_exports_kawi_plugin_lifecycle_init (;14;) (type 8) (param i32) (result i32)
      (local i32)
      global.get $__stack_pointer
      i32.const 32
      i32.sub
      local.tee 1
      global.set $__stack_pointer
      block ;; label = @1
        block ;; label = @2
          local.get 0
          local.get 1
          i32.const 8
          i32.add
          call $exports_kawi_plugin_lifecycle_init
          br_if 0 (;@2;)
          i32.const 0
          local.get 1
          i32.load offset=16
          i32.store offset=65580
          i32.const 0
          i32.const 1
          i32.store8 offset=65568
          i32.const 0
          local.get 1
          i64.load offset=8 align=4
          i64.store offset=65572 align=4
          block ;; label = @3
            local.get 1
            i32.load8_u offset=20
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@3;)
            local.get 1
            i32.load offset=24
            local.set 0
            i32.const 0
            local.get 1
            i32.load offset=28
            i32.store offset=65592
            i32.const 0
            i32.const 1
            i32.store8 offset=65584
            i32.const 0
            local.get 0
            i32.store offset=65588
            br 2 (;@1;)
          end
          i32.const 0
          i32.const 0
          i32.store8 offset=65584
          br 1 (;@1;)
        end
        i32.const 0
        i32.const 0
        i32.store8 offset=65568
      end
      local.get 1
      i32.const 32
      i32.add
      global.set $__stack_pointer
      i32.const 65568
    )
    (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command (;15;) (type 4) (param i32 i32 i32 i32) (result i32)
      (local i32)
      global.get $__stack_pointer
      i32.const 64
      i32.sub
      local.tee 4
      global.set $__stack_pointer
      local.get 4
      local.get 1
      i32.store offset=60
      local.get 4
      local.get 0
      i32.store offset=56
      local.get 4
      local.get 3
      i32.store offset=52
      local.get 4
      local.get 2
      i32.store offset=48
      block ;; label = @1
        block ;; label = @2
          local.get 4
          i32.const 56
          i32.add
          local.get 4
          i32.const 48
          i32.add
          local.get 4
          i32.const 32
          i32.add
          local.get 4
          i32.const 8
          i32.add
          call $exports_kawi_plugin_lifecycle_invoke_command
          br_if 0 (;@2;)
          i32.const 0
          local.get 4
          i32.load offset=16
          i32.store offset=65584
          i32.const 0
          i32.const 1
          i32.store8 offset=65568
          i32.const 0
          local.get 4
          i64.load offset=8 align=4
          i64.store offset=65576
          block ;; label = @3
            local.get 4
            i32.load offset=20
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@3;)
            local.get 4
            i32.load offset=24
            local.set 1
            i32.const 0
            local.get 4
            i32.load offset=28
            i32.store offset=65596
            i32.const 0
            i32.const 1
            i32.store8 offset=65588
            i32.const 0
            local.get 1
            i32.store offset=65592
            br 2 (;@1;)
          end
          i32.const 0
          i32.const 0
          i32.store8 offset=65588
          br 1 (;@1;)
        end
        i32.const 0
        i32.const 0
        i32.store8 offset=65568
        local.get 4
        i32.load offset=44
        local.set 0
        local.get 4
        i32.load offset=40
        local.set 1
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  local.get 4
                  i32.load8_u offset=32
                  br_table 0 (;@6;) 1 (;@5;) 2 (;@4;) 3 (;@3;) 4 (;@2;) 5 (;@1;)
                end
                i32.const 0
                i32.const 0
                i32.store8 offset=65576
                br 4 (;@1;)
              end
              i32.const 0
              i32.const 1
              i32.store8 offset=65576
              i32.const 0
              local.get 1
              i32.const 1
              i32.and
              i32.store8 offset=65584
              br 3 (;@1;)
            end
            i32.const 0
            local.get 0
            i32.store offset=65588
            i32.const 0
            local.get 1
            i32.store offset=65584
            i32.const 0
            i32.const 2
            i32.store8 offset=65576
            br 2 (;@1;)
          end
          i32.const 0
          i32.const 3
          i32.store8 offset=65576
          i32.const 0
          local.get 0
          i64.extend_i32_u
          i64.const 32
          i64.shl
          local.get 1
          i64.extend_i32_u
          i64.or
          i64.store offset=65584
          br 1 (;@1;)
        end
        i32.const 0
        local.get 0
        i32.store offset=65588
        i32.const 0
        i32.const 4
        i32.store8 offset=65576
        i32.const 0
        local.get 1
        i32.store offset=65584
      end
      local.get 4
      i32.const 64
      i32.add
      global.set $__stack_pointer
      i32.const 65568
    )
    (func $__wasm_export_exports_kawi_plugin_lifecycle_on_event (;16;) (type 9) (param i32 i64 i64 i64 i32 i64 i32 i64 i64)
      (local i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32)
      global.get $__stack_pointer
      i32.const 64
      i32.sub
      local.tee 9
      global.set $__stack_pointer
      i64.const 0
      local.set 10
      i32.const 0
      local.set 11
      i32.const 0
      local.set 12
      i32.const 0
      local.set 13
      i32.const 0
      local.set 14
      i32.const 0
      local.set 15
      i32.const 0
      local.set 16
      i32.const 0
      local.set 17
      i32.const 0
      local.set 18
      i64.const 0
      local.set 19
      block ;; label = @1
        block ;; label = @2
          block ;; label = @3
            block ;; label = @4
              block ;; label = @5
                block ;; label = @6
                  block ;; label = @7
                    block ;; label = @8
                      block ;; label = @9
                        block ;; label = @10
                          local.get 0
                          i32.const 255
                          i32.and
                          br_table 0 (;@10;) 1 (;@9;) 2 (;@8;) 3 (;@7;) 4 (;@6;) 5 (;@5;) 6 (;@4;) 9 (;@1;)
                        end
                        local.get 1
                        i64.const -256
                        i64.and
                        local.set 10
                        i32.const 0
                        local.set 12
                        local.get 6
                        i32.const 0
                        i32.ne
                        local.set 11
                        local.get 3
                        i32.wrap_i64
                        local.tee 15
                        i32.const -256
                        i32.and
                        local.set 16
                        local.get 2
                        i32.wrap_i64
                        local.tee 17
                        i32.const -256
                        i32.and
                        local.set 18
                        local.get 3
                        i64.const 40
                        i64.shr_u
                        i32.wrap_i64
                        local.set 14
                        local.get 3
                        i64.const 32
                        i64.shr_u
                        i32.wrap_i64
                        local.set 13
                        local.get 2
                        i64.const 32
                        i64.shr_u
                        i32.wrap_i64
                        local.set 20
                        local.get 8
                        i32.wrap_i64
                        local.set 21
                        local.get 7
                        i32.wrap_i64
                        local.set 22
                        local.get 5
                        i32.wrap_i64
                        local.set 23
                        local.get 1
                        local.set 19
                        br 8 (;@1;)
                      end
                      local.get 4
                      i32.const -256
                      i32.and
                      local.set 16
                      local.get 1
                      i64.const -256
                      i64.and
                      local.set 10
                      local.get 8
                      i32.wrap_i64
                      local.tee 11
                      i32.const -256
                      i32.and
                      local.set 12
                      local.get 5
                      i32.wrap_i64
                      i32.const 1
                      i32.eq
                      local.set 13
                      local.get 2
                      i32.wrap_i64
                      i32.const 1
                      i32.eq
                      local.set 17
                      local.get 8
                      i64.const 32
                      i64.shr_u
                      i32.wrap_i64
                      local.set 22
                      local.get 7
                      i32.wrap_i64
                      local.set 23
                      local.get 3
                      i32.wrap_i64
                      local.set 20
                      local.get 4
                      local.set 15
                      local.get 6
                      local.set 4
                      br 5 (;@3;)
                    end
                    local.get 1
                    i64.const -256
                    i64.and
                    local.set 10
                    local.get 4
                    i32.const 1
                    i32.eq
                    local.set 15
                    local.get 7
                    i32.wrap_i64
                    local.tee 11
                    i32.const -256
                    i32.and
                    local.set 12
                    local.get 5
                    i32.wrap_i64
                    local.tee 13
                    i32.const 8
                    i32.shr_u
                    local.set 14
                    local.get 2
                    i32.wrap_i64
                    local.tee 17
                    i32.const -256
                    i32.and
                    local.set 18
                    local.get 7
                    i64.const 32
                    i64.shr_u
                    i32.wrap_i64
                    local.set 22
                    local.get 3
                    i32.wrap_i64
                    local.set 20
                    i32.const 0
                    local.set 23
                    local.get 6
                    local.set 4
                    i32.const 0
                    local.set 16
                    local.get 1
                    local.set 19
                    br 6 (;@1;)
                  end
                  local.get 4
                  i32.const -256
                  i32.and
                  local.set 16
                  local.get 1
                  i64.const -256
                  i64.and
                  local.set 10
                  local.get 2
                  i32.wrap_i64
                  i32.const 1
                  i32.eq
                  local.set 17
                  local.get 3
                  i32.wrap_i64
                  local.set 20
                  i32.const 0
                  local.set 11
                  i32.const 0
                  local.set 12
                  i32.const 0
                  local.set 13
                  i32.const 0
                  local.set 14
                  local.get 4
                  local.set 15
                  br 4 (;@2;)
                end
                local.get 4
                i32.const -256
                i32.and
                local.set 16
                local.get 1
                i64.const -256
                i64.and
                local.set 10
                local.get 2
                i32.wrap_i64
                i32.const 1
                i32.eq
                local.set 17
                local.get 5
                i64.const 32
                i64.shr_u
                i32.wrap_i64
                local.set 23
                local.get 3
                i32.wrap_i64
                local.set 20
                local.get 4
                local.set 15
                local.get 5
                i32.wrap_i64
                local.set 4
                i32.const 0
                local.set 11
                i32.const 0
                local.set 12
                i32.const 0
                local.set 13
                br 2 (;@3;)
              end
              local.get 4
              i32.const -256
              i32.and
              local.set 16
              local.get 1
              i64.const -256
              i64.and
              local.set 10
              local.get 5
              i32.wrap_i64
              local.tee 13
              i32.const 8
              i32.shr_u
              local.set 14
              local.get 2
              i32.wrap_i64
              local.tee 17
              i32.const -256
              i32.and
              local.set 18
              local.get 3
              i32.wrap_i64
              local.set 20
              i32.const 0
              local.set 11
              i32.const 0
              local.set 12
              local.get 4
              local.set 15
              local.get 1
              local.set 19
              br 3 (;@1;)
            end
            local.get 6
            i32.const -256
            i32.and
            local.set 12
            local.get 4
            i32.const 8
            i32.shr_u
            local.set 14
            local.get 3
            i32.wrap_i64
            local.tee 15
            i32.const -256
            i32.and
            local.set 16
            local.get 2
            i32.wrap_i64
            local.tee 17
            i32.const -256
            i32.and
            local.set 18
            local.get 5
            i64.const 32
            i64.shr_u
            i32.wrap_i64
            local.set 23
            local.get 2
            i64.const 32
            i64.shr_u
            i32.wrap_i64
            local.set 20
            local.get 1
            i32.wrap_i64
            i32.const 1
            i32.eq
            i64.extend_i32_u
            local.set 19
            local.get 7
            i32.wrap_i64
            local.set 22
            local.get 4
            local.set 13
            local.get 5
            i32.wrap_i64
            local.set 4
            local.get 6
            local.set 11
            br 2 (;@1;)
          end
          i32.const 0
          local.set 14
        end
        i32.const 0
        local.set 18
        local.get 1
        local.set 19
      end
      local.get 9
      i32.const 0
      i32.store offset=60
      local.get 9
      local.get 21
      i32.store offset=56
      local.get 9
      local.get 22
      i32.store offset=52
      local.get 9
      local.get 23
      i32.store offset=44
      local.get 9
      local.get 4
      i32.store offset=40
      local.get 9
      local.get 20
      i32.store offset=28
      local.get 9
      local.get 0
      i32.store8 offset=8
      local.get 9
      local.get 12
      local.get 11
      i32.const 255
      i32.and
      i32.or
      i32.store offset=48
      local.get 9
      local.get 14
      i32.const 8
      i32.shl
      local.get 13
      i32.const 255
      i32.and
      i32.or
      i32.store offset=36
      local.get 9
      local.get 16
      local.get 15
      i32.const 255
      i32.and
      i32.or
      i32.store offset=32
      local.get 9
      local.get 18
      local.get 17
      i32.const 255
      i32.and
      i32.or
      i32.store offset=24
      local.get 9
      local.get 10
      local.get 19
      i64.const 255
      i64.and
      i64.or
      i64.store offset=16
      local.get 9
      i32.const 8
      i32.add
      call $exports_kawi_plugin_lifecycle_on_event
      local.get 9
      i32.const 64
      i32.add
      global.set $__stack_pointer
    )
    (func $__component_type_object_force_link_host_public_use_in_this_compilation_unit (;17;) (type 10)
      call $__component_type_object_force_link_host
    )
    (func $realloc (;18;) (type 6) (param i32 i32) (result i32)
      block ;; label = @1
        block ;; label = @2
          local.get 0
          br_if 0 (;@2;)
          local.get 1
          i32.const 65536
          i32.const 0
          i32.load offset=65600
          local.tee 0
          i32.sub
          i32.gt_u
          br_if 1 (;@1;)
          i32.const 0
          local.get 0
          local.get 1
          i32.add
          i32.store offset=65600
          local.get 0
          i32.const 65616
          i32.add
          local.set 0
        end
        local.get 0
        return
      end
      unreachable
    )
    (func $free (;19;) (type 3) (param i32))
    (func $abort (;20;) (type 10)
      unreachable
    )
    (func $exports_kawi_plugin_lifecycle_init (;21;) (type 6) (param i32 i32) (result i32)
      (local i32 i64)
      global.get $__stack_pointer
      i32.const 192
      i32.sub
      local.tee 2
      global.set $__stack_pointer
      block ;; label = @1
        block ;; label = @2
          i32.const 0
          local.get 2
          i32.const 104
          i32.add
          local.get 2
          i32.const 80
          i32.add
          call $kawi_plugin_editor_read_get_state
          i32.eqz
          br_if 0 (;@2;)
          local.get 2
          i32.const 104
          i32.add
          call $kawi_plugin_editor_read_state_free
          br 1 (;@1;)
        end
        local.get 2
        i32.const 80
        i32.add
        call $kawi_plugin_editor_read_error_free
      end
      local.get 2
      i32.const 0
      i64.load offset=65544
      local.tee 3
      i64.store offset=72
      block ;; label = @1
        local.get 2
        i32.const 72
        i32.add
        local.get 2
        i32.const 48
        i32.add
        call $kawi_plugin_ui_message_message
        br_if 0 (;@1;)
        local.get 2
        i32.const 48
        i32.add
        call $kawi_plugin_ui_message_error_free
      end
      local.get 2
      local.get 3
      i64.store offset=40
      local.get 2
      i64.const 0
      i64.store offset=32
      block ;; label = @1
        local.get 2
        i32.const 40
        i32.add
        i64.const 0
        i64.const 0
        local.get 2
        i32.const 32
        i32.add
        local.get 2
        i32.const 8
        i32.add
        call $kawi_plugin_diagnostics_publish_set
        br_if 0 (;@1;)
        local.get 2
        i32.const 8
        i32.add
        call $kawi_plugin_diagnostics_publish_error_free
      end
      local.get 2
      i32.const 192
      i32.add
      global.set $__stack_pointer
      i32.const 1
    )
    (func $exports_kawi_plugin_lifecycle_invoke_command (;22;) (type 4) (param i32 i32 i32 i32) (result i32)
      local.get 2
      i32.const 4
      i32.store offset=12
      local.get 2
      i32.const 65552
      i32.store offset=8
      local.get 2
      i32.const 4
      i32.store8
      i32.const 1
    )
    (func $exports_kawi_plugin_lifecycle_on_event (;23;) (type 3) (param i32))
    (func $__component_type_object_force_link_host (;24;) (type 10))
    (data $.rodata (;0;) (i32.const 65536) "multi\00\00\00\00\00\01\00\05\00\00\00")
    (data $.data (;1;) (i32.const 65552) "pong\00")
    (@producers
      (processed-by "clang" "23.1.3")
      (processed-by "wit-component" "0.262.0")
      (processed-by "wit-bindgen-c" "0.63.0")
    )
    (@custom "target_features" (after data) "\08+\0bbulk-memory+\0fbulk-memory-opt+\16call-indirect-overlong+\0amultivalue+\0fmutable-globals+\13nontrapping-fptoint+\0freference-types+\08sign-ext")
  )
  (core module $wit-component-shim-module (;1;)
    (type (;0;) (func (param i32 i32)))
    (type (;1;) (func (param i32 i32 i32)))
    (type (;2;) (func (param i32 i32 i64 i64 i32 i32 i32)))
    (table (;0;) 3 3 funcref)
    (export "0" (func $indirect-kawi:plugin/editor-read@0.5.0-get-state))
    (export "1" (func $indirect-kawi:plugin/ui-message@0.5.0-message))
    (export "2" (func $indirect-kawi:plugin/diagnostics-publish@0.5.0-set))
    (export "$imports" (table 0))
    (func $indirect-kawi:plugin/editor-read@0.5.0-get-state (;0;) (type 0) (param i32 i32)
      local.get 0
      local.get 1
      i32.const 0
      call_indirect (type 0)
    )
    (func $indirect-kawi:plugin/ui-message@0.5.0-message (;1;) (type 1) (param i32 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      i32.const 1
      call_indirect (type 1)
    )
    (func $indirect-kawi:plugin/diagnostics-publish@0.5.0-set (;2;) (type 2) (param i32 i32 i64 i64 i32 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      local.get 3
      local.get 4
      local.get 5
      local.get 6
      i32.const 2
      call_indirect (type 2)
    )
    (@producers
      (processed-by "wit-component" "0.262.0")
    )
  )
  (core instance $wit-component-shim-instance (;0;) (instantiate $wit-component-shim-module))
  (alias export $kawi:plugin/types@0.5.0 "argument" (type $argument (;9;)))
  (alias export $kawi:plugin/types@0.5.0 "error" (type $"#type10 error" (@name "error") (;10;)))
  (alias export $kawi:plugin/types@0.5.0 "event" (type $event (;11;)))
  (alias core export $wit-component-shim-instance "0" (core func $indirect-kawi:plugin/editor-read@0.5.0-get-state (;0;)))
  (core instance $kawi:plugin/editor-read@0.5.0 (;1;)
    (export "get-state" (func $indirect-kawi:plugin/editor-read@0.5.0-get-state))
  )
  (alias core export $wit-component-shim-instance "1" (core func $indirect-kawi:plugin/ui-message@0.5.0-message (;1;)))
  (core instance $kawi:plugin/ui-message@0.5.0 (;2;)
    (export "message" (func $indirect-kawi:plugin/ui-message@0.5.0-message))
  )
  (alias core export $wit-component-shim-instance "2" (core func $indirect-kawi:plugin/diagnostics-publish@0.5.0-set (;2;)))
  (core instance $kawi:plugin/diagnostics-publish@0.5.0 (;3;)
    (export "set" (func $indirect-kawi:plugin/diagnostics-publish@0.5.0-set))
  )
  (core instance $main (;4;) (instantiate $main
      (with "kawi:plugin/editor-read@0.5.0" (instance $kawi:plugin/editor-read@0.5.0))
      (with "kawi:plugin/ui-message@0.5.0" (instance $kawi:plugin/ui-message@0.5.0))
      (with "kawi:plugin/diagnostics-publish@0.5.0" (instance $kawi:plugin/diagnostics-publish@0.5.0))
    )
  )
  (alias core export $main "memory" (core memory $memory (;0;)))
  (core module $wit-component-fixup (;2;)
    (type (;0;) (func (param i32 i32)))
    (type (;1;) (func (param i32 i32 i32)))
    (type (;2;) (func (param i32 i32 i64 i64 i32 i32 i32)))
    (import "actual" "0" (func $0 (;0;) (type 0) (param i32 i32)))
    (import "actual" "1" (func $1 (;1;) (type 1) (param i32 i32 i32)))
    (import "actual" "2" (func $2 (;2;) (type 2) (param i32 i32 i64 i64 i32 i32 i32)))
    (import "shim" "$imports" (table (;0;) 3 3 funcref))
    (elem (;0;) (i32.const 0) func $0 $1 $2)
    (@producers
      (processed-by "wit-component" "0.262.0")
    )
  )
  (alias export $kawi:plugin/editor-read@0.5.0 "get-state" (func $get-state (;0;)))
  (alias core export $main "cabi_realloc" (core func $cabi_realloc (;3;)))
  (core func $"#core-func4 indirect-kawi:plugin/editor-read@0.5.0-get-state" (@name "indirect-kawi:plugin/editor-read@0.5.0-get-state") (;4;) (canon lower (func $get-state) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (alias export $kawi:plugin/ui-message@0.5.0 "message" (func $message (;1;)))
  (core func $"#core-func5 indirect-kawi:plugin/ui-message@0.5.0-message" (@name "indirect-kawi:plugin/ui-message@0.5.0-message") (;5;) (canon lower (func $message) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (alias export $kawi:plugin/diagnostics-publish@0.5.0 "set" (func $set (;2;)))
  (core func $"#core-func6 indirect-kawi:plugin/diagnostics-publish@0.5.0-set" (@name "indirect-kawi:plugin/diagnostics-publish@0.5.0-set") (;6;) (canon lower (func $set) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (core instance $actual (;5;)
    (export "0" (func $"#core-func4 indirect-kawi:plugin/editor-read@0.5.0-get-state"))
    (export "1" (func $"#core-func5 indirect-kawi:plugin/ui-message@0.5.0-message"))
    (export "2" (func $"#core-func6 indirect-kawi:plugin/diagnostics-publish@0.5.0-set"))
  )
  (core instance $fixup (;6;) (instantiate $wit-component-fixup
      (with "actual" (instance $actual))
      (with "shim" (instance $wit-component-shim-instance))
    )
  )
  (type (;12;) (result (error $"#type10 error")))
  (type (;13;) (func (param "host-protocol" u32) (result 12)))
  (alias core export $main "kawi:plugin/lifecycle@0.5.0#init" (core func $kawi:plugin/lifecycle@0.5.0#init (;7;)))
  (alias core export $main "cabi_post_kawi:plugin/lifecycle@0.5.0#init" (core func $cabi_post_kawi:plugin/lifecycle@0.5.0#init (;8;)))
  (func $init (;3;) (type 13) (canon lift (core func $kawi:plugin/lifecycle@0.5.0#init) (memory $memory) string-encoding=utf8 (post-return $cabi_post_kawi:plugin/lifecycle@0.5.0#init)))
  (type (;14;) (list $argument))
  (type (;15;) (result $argument (error $"#type10 error")))
  (type (;16;) (func (param "id" string) (param "args" 14) (result 15)))
  (alias core export $main "kawi:plugin/lifecycle@0.5.0#invoke-command" (core func $kawi:plugin/lifecycle@0.5.0#invoke-command (;9;)))
  (alias core export $main "cabi_post_kawi:plugin/lifecycle@0.5.0#invoke-command" (core func $cabi_post_kawi:plugin/lifecycle@0.5.0#invoke-command (;10;)))
  (func $invoke-command (;4;) (type 16) (canon lift (core func $kawi:plugin/lifecycle@0.5.0#invoke-command) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8 (post-return $cabi_post_kawi:plugin/lifecycle@0.5.0#invoke-command)))
  (type (;17;) (func (param "event" $event)))
  (alias core export $main "kawi:plugin/lifecycle@0.5.0#on-event" (core func $kawi:plugin/lifecycle@0.5.0#on-event (;11;)))
  (func $on-event (;5;) (type 17) (canon lift (core func $kawi:plugin/lifecycle@0.5.0#on-event) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (alias export $kawi:plugin/types@0.5.0 "position" (type $position (;18;)))
  (alias export $kawi:plugin/types@0.5.0 "edit" (type $edit (;19;)))
  (alias export $kawi:plugin/types@0.5.0 "buffer-changed" (type $buffer-changed (;20;)))
  (alias export $kawi:plugin/types@0.5.0 "buffer-opened" (type $buffer-opened (;21;)))
  (alias export $kawi:plugin/types@0.5.0 "file-opened" (type $file-opened (;22;)))
  (alias export $kawi:plugin/types@0.5.0 "buffer-closed" (type $buffer-closed (;23;)))
  (alias export $kawi:plugin/types@0.5.0 "file-saved" (type $file-saved (;24;)))
  (alias export $kawi:plugin/types@0.5.0 "mode-changed" (type $mode-changed (;25;)))
  (alias export $kawi:plugin/types@0.5.0 "severity" (type $severity (;26;)))
  (alias export $kawi:plugin/types@0.5.0 "diagnostic" (type $"#type27 diagnostic" (@name "diagnostic") (;27;)))
  (alias export $kawi:plugin/types@0.5.0 "diagnostics-changed" (type $diagnostics-changed (;28;)))
  (component $kawi:plugin/lifecycle@0.5.0-shim-component (;0;)
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
  (instance $kawi:plugin/lifecycle@0.5.0-shim-instance (;4;) (instantiate $kawi:plugin/lifecycle@0.5.0-shim-component
      (with "import-func-init" (func $init))
      (with "import-func-invoke-command" (func $invoke-command))
      (with "import-func-on-event" (func $on-event))
      (with "import-type-argument" (type $argument))
      (with "import-type-error" (type $"#type10 error"))
      (with "import-type-position" (type $position))
      (with "import-type-edit" (type $edit))
      (with "import-type-buffer-changed" (type $buffer-changed))
      (with "import-type-buffer-opened" (type $buffer-opened))
      (with "import-type-file-opened" (type $file-opened))
      (with "import-type-buffer-closed" (type $buffer-closed))
      (with "import-type-file-saved" (type $file-saved))
      (with "import-type-mode-changed" (type $mode-changed))
      (with "import-type-severity" (type $severity))
      (with "import-type-diagnostic" (type $"#type27 diagnostic"))
      (with "import-type-diagnostics-changed" (type $diagnostics-changed))
      (with "import-type-event" (type $event))
      (with "import-type-error0" (type $"#type10 error"))
      (with "import-type-argument0" (type $argument))
      (with "import-type-event0" (type $event))
    )
  )
  (export $kawi:plugin/lifecycle@0.5.0 (;5;) "kawi:plugin/lifecycle@0.5.0" (instance $kawi:plugin/lifecycle@0.5.0-shim-instance))
  (@producers
    (processed-by "wit-component" "0.262.0")
  )
)
