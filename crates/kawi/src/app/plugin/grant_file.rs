use std::fs;
use std::path::Path;

use kawi_plugin::{Capability, Scope};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

use super::PluginRuntime;

impl PluginRuntime {
    pub(super) fn write_grant(
        &mut self,
        plugin: &str,
        capability: Capability,
    ) -> Result<(), String> {
        let path = self
            .permissions_path
            .clone()
            .ok_or_else(|| "permissions.toml is not configured".to_string())?;
        let scope = self
            .request_scope(plugin, capability)
            .ok_or_else(|| "capability is not requested".to_string())?;
        let request_hash = self
            .catalog
            .plugins
            .iter()
            .find(|loaded| loaded.manifest.name == plugin)
            .map(|loaded| loaded.manifest.capability_requests.request_hash())
            .ok_or_else(|| "unknown plugin".to_string())?;
        let changed = self.changed_capabilities(plugin);
        write_grant(&path, plugin, capability, &scope, &request_hash, &changed)?;
        self.remember_permission_write(&path);
        Ok(())
    }
}

pub(super) fn write_grant(
    path: &Path,
    plugin: &str,
    capability: Capability,
    scope: &Scope,
    request_hash: &str,
    changed: &[Capability],
) -> Result<(), String> {
    let text = read_permissions(path)?;
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("cannot parse permissions.toml: {error}"))?;
    let plugins = ensure_table(document.as_table_mut(), "plugins")?;
    let plugin_table = ensure_table(plugins, plugin)?;
    plugin_table.insert("request_hash", toml_edit::value(request_hash));
    let grant = ensure_table(plugin_table, "grant")?;
    for changed_capability in changed {
        if *changed_capability != capability {
            grant.remove(changed_capability.name());
        }
    }
    grant.insert(capability.name(), scope_item(scope));
    crate::persist::write_atomically(path, &document.to_string())
        .map_err(|error| format!("cannot write permissions.toml: {error}"))
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
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_file() -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("kawi-grant-file-{}-{id}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join("permissions.toml")
    }

    #[test]
    fn merges_grants_and_preserves_comments() {
        let path = temp_file();
        fs::write(
            &path,
            "# keep\n[plugins.other.grant]\n\"ui.message\" = true\n",
        )
        .unwrap();
        write_grant(
            &path,
            "sample",
            Capability::FsRead,
            &Scope::Paths(vec!["$WORKSPACE/src".to_string()]),
            "sha256:new",
            &[],
        )
        .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# keep"));
        assert!(text.contains("[plugins.other.grant]"));
        assert!(text.contains("[plugins.sample]"));
        assert!(text.contains("request_hash = \"sha256:new\""));
        assert!(text.contains("\"fs.read\" = { paths = [\"$WORKSPACE/src\"] }"));
    }

    #[test]
    fn concurrent_edits_are_re_read_before_merge() {
        let path = temp_file();
        write_grant(
            &path,
            "sample",
            Capability::EditorRead,
            &Scope::Unscoped,
            "sha256:first",
            &[],
        )
        .unwrap();
        fs::write(
            &path,
            format!(
                "{}\n[plugins.other.grant]\n\"ui.message\" = true\n",
                fs::read_to_string(&path).unwrap()
            ),
        )
        .unwrap();
        write_grant(
            &path,
            "sample",
            Capability::EditorWrite,
            &Scope::Unscoped,
            "sha256:second",
            &[],
        )
        .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("[plugins.other.grant]"));
        assert!(text.contains("\"editor.read\" = true"));
        assert!(text.contains("\"editor.write\" = true"));
        assert!(text.contains("request_hash = \"sha256:second\""));
    }
}
