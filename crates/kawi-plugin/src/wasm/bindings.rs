wasmtime::component::bindgen!({
    path: "../kawi-plugin-sdk/wit",
    world: "host",
});

mod callback_bindings {
    wasmtime::component::bindgen!({
        path: "../kawi-plugin-sdk/wit",
        world: "callbacks",
        with: {
            "kawi:plugin/types@0.10.0": super::kawi::plugin::types,
        },
    });
}

pub(crate) use callback_bindings::exports::kawi::plugin::callback_handler::{
    AutocmdCallback, Callback, CallbackEvent,
};
pub(crate) use callback_bindings::Callbacks;
