use kawi_plugin::{Capability, Scope};
use serde_json::{json, Value};

use super::{app, grant, PLUGIN};

#[test]
fn changing_an_option_requests_a_redraw() {
    for (method, params) in [
        ("options.set", json!({ "name": "number", "value": true })),
        ("options.reset", json!({ "name": "number" })),
    ] {
        let mut app = app();
        grant(&mut app, Capability::OptionsWrite, Scope::Unscoped);
        let redraw = app.handle_plugin_host_request(PLUGIN, Value::Null, method, params);
        assert!(redraw, "{method} did not request a redraw");
    }
}
