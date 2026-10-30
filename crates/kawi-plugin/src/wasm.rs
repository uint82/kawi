#[cfg(feature = "wasm")]
mod bindings;
#[cfg(feature = "wasm")]
mod bridge;
#[cfg(feature = "wasm")]
mod byte_budget;
#[cfg(feature = "wasm")]
mod component;
#[cfg(feature = "wasm")]
mod connection_cap;
#[cfg(feature = "wasm")]
mod convert;
#[cfg(feature = "fuzz")]
pub mod fuzz;
#[cfg(feature = "wasm")]
mod inbound_cap;
#[cfg(feature = "wasm")]
mod limits;
#[cfg(feature = "wasm")]
mod name_lookup;
#[cfg(feature = "wasm")]
mod net;
#[cfg(feature = "wasm")]
mod outbound_cap;
#[cfg(feature = "wasm")]
mod policy;
#[cfg(feature = "wasm")]
mod process;
#[cfg(feature = "wasm")]
mod session;
#[cfg(feature = "wasm")]
mod state;
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
mod test_resolver;
#[cfg(all(feature = "wasm", any(test, feature = "fuzz")))]
mod test_streams;
#[cfg(feature = "wasm")]
mod wasi;
#[cfg(feature = "wasm")]
pub(crate) mod worker;

#[cfg(feature = "wasm")]
pub use component::{ComponentInspection, WasmRuntime};
#[cfg(feature = "wasm")]
pub use policy::{
    import_capabilities, import_rule, requested_import_capabilities, unauthorized_imports,
    ImportRule, WASM_IMPORTS,
};
#[cfg(all(feature = "wasm-test-network", debug_assertions))]
pub use test_resolver::{install_resolver_script, remove_resolver_script, resolver_call_count};
#[cfg(feature = "wasm")]
pub use wasi::WasiConfig;
