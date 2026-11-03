use super::server_table;
use crate::ServerConfig;
use std::collections::BTreeMap;

const SERVERS: &[(&str, &str, &[&str])] = &[
    ("marksman", "marksman", &["server"]),
    ("texlab", "texlab", &[]),
    ("tinymist", "tinymist", &[]),
];

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    server_table(SERVERS)
}
