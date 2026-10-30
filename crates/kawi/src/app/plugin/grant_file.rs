use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use kawi_plugin::{reconcile_approved, Capability, CapabilityRequests, Scope};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

use super::PluginRuntime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GrantDecision {
    pub(super) capability: Capability,
    pub(super) allow: bool,
}

impl PluginRuntime {
    pub(super) fn write_grants(
        &mut self,
        plugin: &str,
        decisions: &[GrantDecision],
    ) -> Result<(), String> {
        let path = self
            .permissions_path
            .clone()
            .ok_or_else(|| "permissions.toml is not configured".to_string())?;
        let loaded = self
            .catalog
            .plugins
            .iter()
            .find(|loaded| loaded.manifest.name == plugin)
            .ok_or_else(|| "unknown plugin".to_string())?;
        let requests = loaded.manifest.capability_requests.clone();
        let request_hash = requests.request_hash();
        let approved = self.capabilities.get(plugin).approved;
        write_grants(
            &path,
            plugin,
            &requests,
            &request_hash,
            &approved,
            decisions,
        )?;
        self.remember_permission_write(&path);
        Ok(())
    }
}

pub(super) fn write_grants(
    path: &Path,
    plugin: &str,
    requests: &CapabilityRequests,
    request_hash: &str,
    approved: &BTreeMap<Capability, Scope>,
    decisions: &[GrantDecision],
) -> Result<(), String> {
    let text = read_permissions(path)?;
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("cannot parse permissions.toml: {error}"))?;
    let plugins = ensure_table(document.as_table_mut(), "plugins")?;
    let plugin_table = ensure_table(plugins, plugin)?;
    plugin_table.insert("request_hash", toml_edit::value(request_hash));
    write_approved(plugin_table, approved, requests, decisions)?;
    write_command_gates(plugin_table, requests)?;
    let grant = ensure_table(plugin_table, "grant")?;
    remove_unrequested_grants(grant, requests);
    apply_decisions(grant, requests, decisions)?;
    crate::persist::write_atomically(path, &document.to_string())
        .map_err(|error| format!("cannot write permissions.toml: {error}"))
}

fn write_approved(
    plugin_table: &mut Table,
    approved: &BTreeMap<Capability, Scope>,
    requests: &CapabilityRequests,
    decisions: &[GrantDecision],
) -> Result<(), String> {
    let decision_map = decisions
        .iter()
        .map(|decision| (decision.capability, decision.allow))
        .collect::<BTreeMap<_, _>>();
    let next = reconcile_approved(approved, requests, &decision_map);
    let table = ensure_table(plugin_table, "approved")?;
    *table = Table::new();
    for (capability, scope) in next {
        table.insert(capability.name(), scope_item(&scope));
    }
    Ok(())
}

fn write_command_gates(
    plugin_table: &mut Table,
    requests: &CapabilityRequests,
) -> Result<(), String> {
    if requests.command_gates().is_empty() {
        plugin_table.remove("command_gates");
        return Ok(());
    }
    let table = ensure_table(plugin_table, "command_gates")?;
    *table = Table::new();
    for (command, capabilities) in requests.command_gates() {
        let mut array = Array::new();
        for capability in capabilities {
            array.push(capability.name());
        }
        table.insert(command, Item::Value(Value::Array(array)));
    }
    Ok(())
}

fn apply_decisions(
    grant: &mut Table,
    requests: &CapabilityRequests,
    decisions: &[GrantDecision],
) -> Result<(), String> {
    for decision in decisions {
        if decision.allow {
            let scope = requests
                .get(decision.capability)
                .ok_or_else(|| "capability is not requested".to_string())?;
            grant.insert(decision.capability.name(), scope_item(scope));
        } else {
            grant.remove(decision.capability.name());
        }
    }
    Ok(())
}

fn remove_unrequested_grants(grant: &mut Table, requests: &CapabilityRequests) {
    let stale = grant
        .iter()
        .filter_map(|(name, _)| Capability::parse(name))
        .filter(|capability| !requests.contains(*capability))
        .collect::<Vec<_>>();
    for capability in stale {
        grant.remove(capability.name());
    }
}

fn read_permissions(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("cannot read permissions.toml: {error}")),
    }
}

fn ensure_table<'a>(table: &'a mut Table, key: &str) -> Result<&'a mut Table, String> {
    if !table.contains_key(key) {
        table.insert(key, Item::Table(Table::new()));
    }
    table[key]
        .as_table_mut()
        .ok_or_else(|| format!("{key} must be a table"))
}

fn scope_item(scope: &Scope) -> Item {
    match scope {
        Scope::Unscoped => Item::Value(Value::from(true)),
        Scope::Paths(paths) => scoped_item("paths", paths),
        Scope::Commands(commands) => scoped_item("commands", commands),
        Scope::Hosts(hosts) => scoped_item("hosts", hosts),
    }
}

fn scoped_item(key: &str, values: &[String]) -> Item {
    let mut table = InlineTable::new();
    let array = values
        .iter()
        .map(|value| Value::from(value.as_str()))
        .collect::<Array>();
    table.insert(key, Value::Array(array));
    Item::Value(Value::InlineTable(table))
}

#[cfg(test)]
mod tests;
