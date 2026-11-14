(component
  (type $ty-kawi:plugin/types@0.8.0 (;0;)
    (instance
      (type (;0;) (variant (case "null") (case "boolean" bool) (case "integer" s64) (case "float" f64) (case "text" string)))
      (export (;1;) "argument" (type (eq 0)))
      (type (;2;) (option string))
      (type (;3;) (record (field "code" s32) (field "message" string) (field "capability" 2)))
      (export (;4;) "error" (type (eq 3)))
      (type (;5;) (record (field "line" u32) (field "byte-column" u32)))
      (export (;6;) "position" (type (eq 5)))
      (type (;7;) (record (field "start" 6) (field "end" 6) (field "text" string)))
      (export (;8;) "edit" (type (eq 7)))
      (type (;9;) (list 8))
      (type (;10;) (record (field "buffer" u64) (field "revision" u64) (field "previous-revision" u64) (field "origin" string) (field "full-resync" bool) (field "changes" 9)))
      (export (;11;) "buffer-changed" (type (eq 10)))
      (type (;12;) (record (field "buffer" u64) (field "path" 2) (field "language" 2) (field "revision" u64)))
      (export (;13;) "buffer-opened" (type (eq 12)))
      (type (;14;) (record (field "buffer" u64) (field "path" string) (field "language" 2) (field "revision" u64)))
      (export (;15;) "file-opened" (type (eq 14)))
      (type (;16;) (record (field "buffer" u64) (field "path" 2)))
      (export (;17;) "buffer-closed" (type (eq 16)))
      (type (;18;) (record (field "buffer" u64) (field "path" 2) (field "revision" u64)))
      (export (;19;) "file-saved" (type (eq 18)))
      (type (;20;) (record (field "buffer" u64) (field "previous" string) (field "next" string)))
      (export (;21;) "mode-changed" (type (eq 20)))
      (type (;22;) (enum "error" "warning" "information" "hint"))
      (export (;23;) "severity" (type (eq 22)))
      (type (;24;) (record (field "start" 6) (field "end" 6) (field "severity" 23) (field "message" string) (field "source" 2) (field "code" 2)))
      (export (;25;) "diagnostic" (type (eq 24)))
      (type (;26;) (option u64))
      (type (;27;) (list 25))
      (type (;28;) (record (field "buffer" 26) (field "path" string) (field "revision" u64) (field "diagnostics" 27)))
      (export (;29;) "diagnostics-changed" (type (eq 28)))
      (type (;30;) (record (field "buffer" u64) (field "path" 2) (field "language" 2) (field "revision" u64)))
      (export (;31;) "buffer-enter" (type (eq 30)))
      (type (;32;) (record (field "buffer" u64) (field "path" 2)))
      (export (;33;) "buffer-leave" (type (eq 32)))
      (type (;34;) (record (field "window" u64) (field "buffer" u64)))
      (export (;35;) "window-enter" (type (eq 34)))
      (type (;36;) (record (field "window" u64) (field "buffer" u64)))
      (export (;37;) "window-leave" (type (eq 36)))
      (type (;38;) (record (field "start" 6) (field "end" 6)))
      (export (;39;) "selection" (type (eq 38)))
      (type (;40;) (option 39))
      (type (;41;) (record (field "buffer" u64) (field "cursor" 6) (field "selection" 40)))
      (export (;42;) "cursor-moved" (type (eq 41)))
      (type (;43;) (record (field "buffer" u64) (field "selection" 40)))
      (export (;44;) "selection-changed" (type (eq 43)))
      (type (;45;) (record (field "window" u64) (field "buffer" u64)))
      (export (;46;) "focus-changed" (type (eq 45)))
      (type (;47;) (variant (case "buffer-changed" 11) (case "buffer-opened" 13) (case "file-opened" 15) (case "buffer-closed" 17) (case "file-saved" 19) (case "mode-changed" 21) (case "diagnostics-changed" 29) (case "buffer-enter" 31) (case "buffer-leave" 33) (case "window-enter" 35) (case "window-leave" 37) (case "cursor-moved" 42) (case "selection-changed" 44) (case "focus-gained" 46) (case "focus-lost" 46)))
      (export (;48;) "event" (type (eq 47)))
    )
  )
  (import "kawi:plugin/types@0.8.0" (instance $kawi:plugin/types@0.8.0 (;0;) (type $ty-kawi:plugin/types@0.8.0)))
  (core module $main (;0;)
    (type (;0;) (func (param i32)))
    (type (;1;) (func (param i32 i32 i32 i32) (result i32)))
    (type (;2;) (func (param i32) (result i32)))
    (type (;3;) (func (param i32 i64 i64 i64 i32 i64 i32 i64 i64)))
    (type (;4;) (func))
    (type (;5;) (func (param i32 i32) (result i32)))
    (table (;0;) 1 1 funcref)
    (memory (;0;) 3)
    (global $__stack_pointer (;0;) (mut i32) i32.const 65536)
    (export "memory" (memory 0))
    (export "cabi_post_kawi:plugin/lifecycle@0.8.0#init" (func $__wasm_export_exports_kawi_plugin_lifecycle_init_post_return))
    (export "cabi_post_kawi:plugin/lifecycle@0.8.0#invoke-command" (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command_post_return))
    (export "cabi_realloc" (func $cabi_realloc))
    (export "kawi:plugin/lifecycle@0.8.0#init" (func $__wasm_export_exports_kawi_plugin_lifecycle_init))
    (export "kawi:plugin/lifecycle@0.8.0#invoke-command" (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command))
    (export "kawi:plugin/lifecycle@0.8.0#on-event" (func $__wasm_export_exports_kawi_plugin_lifecycle_on_event))
    (func $__wasm_export_exports_kawi_plugin_lifecycle_init_post_return (;0;) (type 0) (param i32)
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
    (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command_post_return (;1;) (type 0) (param i32)
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
    (func $cabi_realloc (;2;) (type 1) (param i32 i32 i32 i32) (result i32)
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
        i32.const 0
        local.set 2
      end
      local.get 2
    )
    (func $__wasm_export_exports_kawi_plugin_lifecycle_init (;3;) (type 2) (param i32) (result i32)
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
          i32.store offset=65548
          i32.const 0
          i32.const 1
          i32.store8 offset=65536
          i32.const 0
          local.get 1
          i64.load offset=8 align=4
          i64.store offset=65540 align=4
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
            i32.store offset=65560
            i32.const 0
            i32.const 1
            i32.store8 offset=65552
            i32.const 0
            local.get 0
            i32.store offset=65556
            br 2 (;@1;)
          end
          i32.const 0
          i32.const 0
          i32.store8 offset=65552
          br 1 (;@1;)
        end
        i32.const 0
        i32.const 0
        i32.store8 offset=65536
      end
      local.get 1
      i32.const 32
      i32.add
      global.set $__stack_pointer
      i32.const 65536
    )
    (func $__wasm_export_exports_kawi_plugin_lifecycle_invoke_command (;4;) (type 1) (param i32 i32 i32 i32) (result i32)
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
          i32.store offset=65552
          i32.const 0
          i32.const 1
          i32.store8 offset=65536
          i32.const 0
          local.get 4
          i64.load offset=8 align=4
          i64.store offset=65544
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
            i32.store offset=65564
            i32.const 0
            i32.const 1
            i32.store8 offset=65556
            i32.const 0
            local.get 1
            i32.store offset=65560
            br 2 (;@1;)
          end
          i32.const 0
          i32.const 0
          i32.store8 offset=65556
          br 1 (;@1;)
        end
        i32.const 0
        i32.const 0
        i32.store8 offset=65536
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
                i32.store8 offset=65544
                br 4 (;@1;)
              end
              i32.const 0
              i32.const 1
              i32.store8 offset=65544
              i32.const 0
              local.get 1
              i32.const 1
              i32.and
              i32.store8 offset=65552
              br 3 (;@1;)
            end
            i32.const 0
            local.get 0
            i32.store offset=65556
            i32.const 0
            local.get 1
            i32.store offset=65552
            i32.const 0
            i32.const 2
            i32.store8 offset=65544
            br 2 (;@1;)
          end
          i32.const 0
          i32.const 3
          i32.store8 offset=65544
          i32.const 0
          local.get 0
          i64.extend_i32_u
          i64.const 32
          i64.shl
          local.get 1
          i64.extend_i32_u
          i64.or
          i64.store offset=65552
          br 1 (;@1;)
        end
        i32.const 0
        local.get 0
        i32.store offset=65556
        i32.const 0
        i32.const 4
        i32.store8 offset=65544
        i32.const 0
        local.get 1
        i32.store offset=65552
      end
      local.get 4
      i32.const 64
      i32.add
      global.set $__stack_pointer
      i32.const 65536
    )
    (func $__wasm_export_exports_kawi_plugin_lifecycle_on_event (;5;) (type 3) (param i32 i64 i64 i64 i32 i64 i32 i64 i64)
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
    (func $__component_type_object_force_link_guest_public_use_in_this_compilation_unit (;6;) (type 4)
      call $__component_type_object_force_link_guest
    )
    (func $realloc (;7;) (type 5) (param i32 i32) (result i32)
      block ;; label = @1
        local.get 0
        br_if 0 (;@1;)
        i32.const 0
        i32.const 0
        i32.load offset=131104
        local.tee 0
        local.get 1
        i32.add
        i32.store offset=131104
        local.get 0
        i32.const 65568
        i32.add
        local.set 0
      end
      local.get 0
    )
    (func $free (;8;) (type 0) (param i32))
    (func $abort (;9;) (type 4)
      unreachable
    )
    (func $exports_kawi_plugin_lifecycle_init (;10;) (type 5) (param i32 i32) (result i32)
      i32.const 1
    )
    (func $exports_kawi_plugin_lifecycle_invoke_command (;11;) (type 1) (param i32 i32 i32 i32) (result i32)
      loop (result i32) ;; label = @1
        br 0 (;@1;)
      end
    )
    (func $exports_kawi_plugin_lifecycle_on_event (;12;) (type 0) (param i32))
    (func $__component_type_object_force_link_guest (;13;) (type 4))
    (@custom "target_features" (after code) "\08+\0bbulk-memory+\0fbulk-memory-opt+\16call-indirect-overlong+\0amultivalue+\0fmutable-globals+\13nontrapping-fptoint+\0freference-types+\08sign-ext")
    (@producers
      (processed-by "clang" "23.1.3")
      (processed-by "wit-component" "0.262.0")
      (processed-by "wit-bindgen-c" "0.63.0")
    )
  )
  (alias export $kawi:plugin/types@0.8.0 "argument" (type $argument (;1;)))
  (alias export $kawi:plugin/types@0.8.0 "error" (type $error (;2;)))
  (alias export $kawi:plugin/types@0.8.0 "event" (type $event (;3;)))
  (core instance $main (;0;) (instantiate $main))
  (alias core export $main "memory" (core memory $memory (;0;)))
  (type (;4;) (result (error $error)))
  (type (;5;) (func (param "host-protocol" u32) (result 4)))
  (alias core export $main "kawi:plugin/lifecycle@0.8.0#init" (core func $kawi:plugin/lifecycle@0.8.0#init (;0;)))
  (alias core export $main "cabi_post_kawi:plugin/lifecycle@0.8.0#init" (core func $cabi_post_kawi:plugin/lifecycle@0.8.0#init (;1;)))
  (func $init (;0;) (type 5) (canon lift (core func $kawi:plugin/lifecycle@0.8.0#init) (memory $memory) string-encoding=utf8 (post-return $cabi_post_kawi:plugin/lifecycle@0.8.0#init)))
  (type (;6;) (list $argument))
  (type (;7;) (result $argument (error $error)))
  (type (;8;) (func (param "id" string) (param "args" 6) (result 7)))
  (alias core export $main "kawi:plugin/lifecycle@0.8.0#invoke-command" (core func $kawi:plugin/lifecycle@0.8.0#invoke-command (;2;)))
  (alias core export $main "cabi_realloc" (core func $cabi_realloc (;3;)))
  (alias core export $main "cabi_post_kawi:plugin/lifecycle@0.8.0#invoke-command" (core func $cabi_post_kawi:plugin/lifecycle@0.8.0#invoke-command (;4;)))
  (func $invoke-command (;1;) (type 8) (canon lift (core func $kawi:plugin/lifecycle@0.8.0#invoke-command) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8 (post-return $cabi_post_kawi:plugin/lifecycle@0.8.0#invoke-command)))
  (type (;9;) (func (param "event" $event)))
  (alias core export $main "kawi:plugin/lifecycle@0.8.0#on-event" (core func $kawi:plugin/lifecycle@0.8.0#on-event (;5;)))
  (func $on-event (;2;) (type 9) (canon lift (core func $kawi:plugin/lifecycle@0.8.0#on-event) (memory $memory) (realloc $cabi_realloc) string-encoding=utf8))
  (alias export $kawi:plugin/types@0.8.0 "position" (type $position (;10;)))
  (alias export $kawi:plugin/types@0.8.0 "edit" (type $edit (;11;)))
  (alias export $kawi:plugin/types@0.8.0 "buffer-changed" (type $buffer-changed (;12;)))
  (alias export $kawi:plugin/types@0.8.0 "buffer-opened" (type $buffer-opened (;13;)))
  (alias export $kawi:plugin/types@0.8.0 "file-opened" (type $file-opened (;14;)))
  (alias export $kawi:plugin/types@0.8.0 "buffer-closed" (type $buffer-closed (;15;)))
  (alias export $kawi:plugin/types@0.8.0 "file-saved" (type $file-saved (;16;)))
  (alias export $kawi:plugin/types@0.8.0 "mode-changed" (type $mode-changed (;17;)))
  (alias export $kawi:plugin/types@0.8.0 "severity" (type $severity (;18;)))
  (alias export $kawi:plugin/types@0.8.0 "diagnostic" (type $diagnostic (;19;)))
  (alias export $kawi:plugin/types@0.8.0 "diagnostics-changed" (type $diagnostics-changed (;20;)))
  (alias export $kawi:plugin/types@0.8.0 "buffer-enter" (type $buffer-enter (;21;)))
  (alias export $kawi:plugin/types@0.8.0 "buffer-leave" (type $buffer-leave (;22;)))
  (alias export $kawi:plugin/types@0.8.0 "window-enter" (type $window-enter (;23;)))
  (alias export $kawi:plugin/types@0.8.0 "window-leave" (type $window-leave (;24;)))
  (alias export $kawi:plugin/types@0.8.0 "selection" (type $selection (;25;)))
  (alias export $kawi:plugin/types@0.8.0 "cursor-moved" (type $cursor-moved (;26;)))
  (alias export $kawi:plugin/types@0.8.0 "selection-changed" (type $selection-changed (;27;)))
  (alias export $kawi:plugin/types@0.8.0 "focus-changed" (type $focus-changed (;28;)))
  (component $kawi:plugin/lifecycle@0.8.0-shim-component (;0;)
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
    (type (;30;) (record (field "buffer" u64) (field "path" 2) (field "language" 2) (field "revision" u64)))
    (import "import-type-buffer-enter" (type (;31;) (eq 30)))
    (type (;32;) (record (field "buffer" u64) (field "path" 2)))
    (import "import-type-buffer-leave" (type (;33;) (eq 32)))
    (type (;34;) (record (field "window" u64) (field "buffer" u64)))
    (import "import-type-window-enter" (type (;35;) (eq 34)))
    (type (;36;) (record (field "window" u64) (field "buffer" u64)))
    (import "import-type-window-leave" (type (;37;) (eq 36)))
    (type (;38;) (record (field "start" 6) (field "end" 6)))
    (import "import-type-selection" (type (;39;) (eq 38)))
    (type (;40;) (option 39))
    (type (;41;) (record (field "buffer" u64) (field "cursor" 6) (field "selection" 40)))
    (import "import-type-cursor-moved" (type (;42;) (eq 41)))
    (type (;43;) (record (field "buffer" u64) (field "selection" 40)))
    (import "import-type-selection-changed" (type (;44;) (eq 43)))
    (type (;45;) (record (field "window" u64) (field "buffer" u64)))
    (import "import-type-focus-changed" (type (;46;) (eq 45)))
    (type (;47;) (variant (case "buffer-changed" 11) (case "buffer-opened" 13) (case "file-opened" 15) (case "buffer-closed" 17) (case "file-saved" 19) (case "mode-changed" 21) (case "diagnostics-changed" 29) (case "buffer-enter" 31) (case "buffer-leave" 33) (case "window-enter" 35) (case "window-leave" 37) (case "cursor-moved" 42) (case "selection-changed" 44) (case "focus-gained" 46) (case "focus-lost" 46)))
    (import "import-type-event" (type (;48;) (eq 47)))
    (import "import-type-error0" (type (;49;) (eq 4)))
    (type (;50;) (result (error 49)))
    (type (;51;) (func (param "host-protocol" u32) (result 50)))
    (import "import-func-init" (func (;0;) (type 51)))
    (import "import-type-argument0" (type (;52;) (eq 1)))
    (type (;53;) (list 52))
    (type (;54;) (result 52 (error 49)))
    (type (;55;) (func (param "id" string) (param "args" 53) (result 54)))
    (import "import-func-invoke-command" (func (;1;) (type 55)))
    (import "import-type-event0" (type (;56;) (eq 48)))
    (type (;57;) (func (param "event" 56)))
    (import "import-func-on-event" (func (;2;) (type 57)))
    (export (;58;) "argument" (type 1))
    (export (;59;) "error" (type 4))
    (export (;60;) "event" (type 48))
    (type (;61;) (result (error 59)))
    (type (;62;) (func (param "host-protocol" u32) (result 61)))
    (export (;3;) "init" (func 0) (func (type 62)))
    (type (;63;) (list 58))
    (type (;64;) (result 58 (error 59)))
    (type (;65;) (func (param "id" string) (param "args" 63) (result 64)))
    (export (;4;) "invoke-command" (func 1) (func (type 65)))
    (type (;66;) (func (param "event" 60)))
    (export (;5;) "on-event" (func 2) (func (type 66)))
  )
  (instance $kawi:plugin/lifecycle@0.8.0-shim-instance (;1;) (instantiate $kawi:plugin/lifecycle@0.8.0-shim-component
      (with "import-func-init" (func $init))
      (with "import-func-invoke-command" (func $invoke-command))
      (with "import-func-on-event" (func $on-event))
      (with "import-type-argument" (type $argument))
      (with "import-type-error" (type $error))
      (with "import-type-position" (type $position))
      (with "import-type-edit" (type $edit))
      (with "import-type-buffer-changed" (type $buffer-changed))
      (with "import-type-buffer-opened" (type $buffer-opened))
      (with "import-type-file-opened" (type $file-opened))
      (with "import-type-buffer-closed" (type $buffer-closed))
      (with "import-type-file-saved" (type $file-saved))
      (with "import-type-mode-changed" (type $mode-changed))
      (with "import-type-severity" (type $severity))
      (with "import-type-diagnostic" (type $diagnostic))
      (with "import-type-diagnostics-changed" (type $diagnostics-changed))
      (with "import-type-buffer-enter" (type $buffer-enter))
      (with "import-type-buffer-leave" (type $buffer-leave))
      (with "import-type-window-enter" (type $window-enter))
      (with "import-type-window-leave" (type $window-leave))
      (with "import-type-selection" (type $selection))
      (with "import-type-cursor-moved" (type $cursor-moved))
      (with "import-type-selection-changed" (type $selection-changed))
      (with "import-type-focus-changed" (type $focus-changed))
      (with "import-type-event" (type $event))
      (with "import-type-error0" (type $error))
      (with "import-type-argument0" (type $argument))
      (with "import-type-event0" (type $event))
    )
  )
  (export $kawi:plugin/lifecycle@0.8.0 (;2;) "kawi:plugin/lifecycle@0.8.0" (instance $kawi:plugin/lifecycle@0.8.0-shim-instance))
  (@producers
    (processed-by "wit-component" "0.262.0")
  )
)
