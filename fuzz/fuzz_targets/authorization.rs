#![no_main]

use kawi_fuzz::host::registry;
use kawi_plugin::{
    advertised_host_methods, host_method_capability, is_host_method_allowed, Capability,
    MethodCapability,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines();
    let grants = lines
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let registry = registry(grants);
    let advertised = advertised_host_methods(&registry, "fuzz");

    for method in lines.map(str::trim).filter(|method| !method.is_empty()) {
        let allowed = is_host_method_allowed(&registry, "fuzz", method);
        match host_method_capability(method) {
            MethodCapability::Required(capability) => {
                assert_eq!(allowed, granted(grants, capability));
            }
            MethodCapability::None => assert!(allowed),
            MethodCapability::Unknown => assert!(!allowed),
        }
        assert_eq!(advertised.contains(&method), allowed);
    }
});

fn granted(grants: u32, capability: Capability) -> bool {
    let index = Capability::ALL
        .iter()
        .position(|candidate| *candidate == capability)
        .expect("capability");
    grants & (1 << index) != 0
}
