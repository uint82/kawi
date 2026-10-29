mod core_parity;
mod faults;
mod network;
mod payloads;
mod wasm;

use std::sync::LazyLock;

use super::Scenario;

static SCENARIOS: LazyLock<Vec<Scenario>> = LazyLock::new(|| {
    let mut rows = Vec::new();
    rows.extend_from_slice(core_parity::ROWS);
    rows.extend_from_slice(faults::ROWS);
    rows.extend_from_slice(wasm::ROWS);
    rows.extend_from_slice(payloads::ROWS);
    rows.extend_from_slice(network::ROWS);
    rows
});

pub(crate) fn scenarios() -> &'static [Scenario] {
    &SCENARIOS
}
