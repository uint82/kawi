use kawi_plugin::Capability;

use crate::plugin_labels::WASM_NETWORK_ENFORCEMENT;

pub(super) fn describe_capabilities(
    capabilities: impl Iterator<Item = Capability>,
    wasm: bool,
    sandboxed: bool,
) -> String {
    let names = capabilities
        .map(|capability| describe_capability(capability, wasm, sandboxed))
        .collect::<Vec<_>>();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

fn describe_capability(capability: Capability, wasm: bool, sandboxed: bool) -> String {
    if capability == Capability::NetConnect {
        let enforcement = if wasm {
            WASM_NETWORK_ENFORCEMENT
        } else {
            "advisory"
        };
        return format!("{} ({enforcement})", capability.name());
    }
    if capability.is_advisory() && sandboxed {
        return format!("{} (enforced)", capability.name());
    }
    if capability.is_advisory() {
        return format!("{} (advisory)", capability.name());
    }
    capability.name().to_string()
}

pub(super) fn denial_lines(capability: Capability) -> [String; 2] {
    [
        format!("denied {}", capability.name()),
        format!(
            "{} is not declared by the plugin manifest",
            capability.name()
        ),
    ]
}
