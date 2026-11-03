use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[("pyright", "pyright-langserver", &["--stdio"])];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
