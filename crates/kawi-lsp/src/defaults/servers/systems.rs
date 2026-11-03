use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[("rust-analyzer", "rust-analyzer", &[])];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
