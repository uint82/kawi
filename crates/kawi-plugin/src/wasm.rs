#[cfg(feature = "wasm")]
mod bindings;
#[cfg(feature = "wasm")]
mod component;
#[cfg(feature = "wasm")]
mod policy;
#[cfg(feature = "wasm")]
mod wasi;

#[cfg(feature = "wasm")]
pub use component::{ComponentInspection, WasmRuntime};
#[cfg(feature = "wasm")]
pub use policy::{
    import_capabilities, import_rule, requested_import_capabilities, unauthorized_imports,
    ImportRule, WASM_IMPORTS,
};
#[cfg(feature = "wasm")]
pub use wasi::WasiConfig;
