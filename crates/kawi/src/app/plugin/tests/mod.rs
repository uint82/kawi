mod common;
mod composition;
mod core;
mod end_to_end;
mod events;
mod extensions;
mod host_api;
mod jobs;
mod manager;
mod panel;
mod process_panels;
mod queue;
mod reload;
mod settings;
mod settings_ui;
mod ui;
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
