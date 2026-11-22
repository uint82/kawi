use super::apply_edit::{BUFFER_METHOD, EDITOR_METHOD};
use crate::capability::{host_method_capability, Capability, MethodCapability};

#[test]
fn apply_edit_gateways_target_distinct_methods() {
    assert_ne!(EDITOR_METHOD, BUFFER_METHOD);
}

#[test]
fn editor_write_gateway_targets_the_unscoped_method() {
    assert_eq!(EDITOR_METHOD, "editor.apply_edit");
    assert_eq!(
        host_method_capability(EDITOR_METHOD),
        MethodCapability::Required(Capability::EditorWrite)
    );
}

#[test]
fn buffers_write_gateway_targets_the_path_scoped_method() {
    assert_eq!(BUFFER_METHOD, "buffer.apply_edit");
    assert_eq!(
        host_method_capability(BUFFER_METHOD),
        MethodCapability::Required(Capability::EditorBuffersWrite)
    );
}
