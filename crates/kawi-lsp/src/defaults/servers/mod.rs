use crate::ServerConfig;
use std::collections::BTreeMap;

mod jvm_dotnet;
mod scripting;
mod systems;
mod web;

pub(super) fn servers() -> BTreeMap<String, ServerConfig> {
    let mut servers = BTreeMap::new();
    servers.extend(systems::servers());
    servers.extend(web::servers());
    servers.extend(scripting::servers());
    servers.extend(jvm_dotnet::servers());
    servers
}

pub(super) fn server_table(entries: &[(&str, &str, &[&str])]) -> BTreeMap<String, ServerConfig> {
    entries
        .iter()
        .map(|(name, command, args)| ((*name).to_string(), server(command, args)))
        .collect()
}

fn server(command: &str, args: &[&str]) -> ServerConfig {
    let mut server = ServerConfig::command(command);
    server.args = args.iter().map(|arg| (*arg).to_string()).collect();
    server
}
