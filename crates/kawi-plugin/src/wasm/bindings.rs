wasmtime::component::bindgen!({
    path: "../kawi-plugin-sdk/wit",
    world: "host",
});

mod callback_bindings {
    wasmtime::component::bindgen!({
        path: "../kawi-plugin-sdk/wit",
        world: "callbacks",
        with: {
            "kawi:plugin/types@0.12.0": super::kawi::plugin::types,
        },
    });
}

pub(crate) use callback_bindings::exports::kawi::plugin::callback_handler::{
    AutocmdCallback, Callback, CallbackEvent, TimerFired,
};
pub(crate) use callback_bindings::Callbacks;
