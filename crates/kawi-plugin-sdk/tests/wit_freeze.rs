use sha2::{Digest, Sha256};

const WIT: &[u8] = include_bytes!("../wit/plugin.wit");
const FROZEN_SHA256: &str = "deee8b0814846363e6345d2c3db2b870f03afbd92f52cb68ca1c5ed444a1cd23";

#[test]
fn plugin_wit_is_frozen() {
    let digest = Sha256::digest(WIT);
    assert_eq!(format!("{digest:x}"), FROZEN_SHA256);
}

#[test]
fn plugin_wit_stays_on_0_13_0() {
    let source = std::str::from_utf8(WIT).expect("wit is utf-8");
    assert!(source.starts_with("package kawi:plugin@0.13.0;"));
}
