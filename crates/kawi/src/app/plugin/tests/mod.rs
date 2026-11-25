mod autocmds;
mod common;
mod composition;
mod core;
mod extensions;
mod host_api;
mod manager;
mod panel;
mod reload;
mod settings;
mod settings_ui;
mod visual;
mod wasm;
#[cfg(feature = "wasm")]
mod wasm_automation;
#[cfg(feature = "wasm")]
mod wasm_composition;
#[cfg(feature = "wasm")]
mod wasm_extensions;
#[cfg(all(feature = "wasm", unix))]
mod wasm_jobs;
#[cfg(all(feature = "wasm", unix))]
mod wasm_panels;
#[cfg(all(feature = "wasm", unix))]
mod wasm_tasks;
