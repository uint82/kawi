use super::*;

impl DebugConfig {
    pub(super) fn load_file(&mut self, path: &Path) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => {
                self.warnings
                    .push(DebugWarning::new(path, format!("rejected {error}")));
                return;
            }
        };
        let value = match toml::from_str::<toml::Value>(&text) {
            Ok(value) => value,
            Err(error) => {
                self.warnings
                    .push(DebugWarning::new(path, format!("rejected {error}")));
                return;
            }
        };
        let Some(table) = value.as_table() else {
            self.warnings
                .push(DebugWarning::new(path, "rejected: expected a table"));
            return;
        };
        for (key, item) in table {
            match key.as_str() {
                "adapter" => self.load_adapters(path, item),
                "configuration" => self.load_configurations(path, item),
                other => self
                    .warnings
                    .push(DebugWarning::new(path, format!("{other}: ignored"))),
            }
        }
    }

    fn load_adapters(&mut self, path: &Path, value: &toml::Value) {
        let Some(table) = value.as_table() else {
            self.warnings.push(DebugWarning::new(
                path,
                "adapter: rejected: expected a table of adapters",
            ));
            return;
        };
        for (name, item) in table {
            let Some(adapter) = load_adapter(path, name, item, &mut self.warnings) else {
                continue;
            };
            self.adapters.insert(name.clone(), adapter);
        }
    }

    fn load_configurations(&mut self, path: &Path, value: &toml::Value) {
        let Some(table) = value.as_table() else {
            self.warnings.push(DebugWarning::new(
                path,
                "configuration: rejected: expected a table of configurations",
            ));
            return;
        };
        for (name, item) in table {
            let Some(configuration) = load_configuration(path, name, item, &mut self.warnings)
            else {
                continue;
            };
            self.configurations.insert(name.clone(), configuration);
        }
    }
}

fn load_adapter(
    path: &Path,
    name: &str,
    value: &toml::Value,
    warnings: &mut Vec<DebugWarning>,
) -> Option<AdapterConfig> {
    let Some(table) = value.as_table() else {
        warnings.push(DebugWarning::new(
            path,
            format!("adapter {name}: rejected: expected a table"),
        ));
        return None;
    };
    warn_unknown(
        path,
        &format!("adapter {name}"),
        table.keys(),
        ADAPTER_FIELDS,
        warnings,
    );
    let command = read_string(path, &format!("adapter {name}"), table, "command", warnings)?;
    let args = read_strings(path, &format!("adapter {name}"), table, "args", warnings);
    let env = read_env(path, &format!("adapter {name}"), table, warnings);
    let cwd = table
        .get("cwd")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    Some(AdapterConfig {
        command,
        args,
        env,
        cwd,
    })
}

fn load_configuration(
    path: &Path,
    name: &str,
    value: &toml::Value,
    warnings: &mut Vec<DebugWarning>,
) -> Option<DebugConfiguration> {
    let Some(table) = value.as_table() else {
        warnings.push(DebugWarning::new(
            path,
            format!("configuration {name}: rejected: expected a table"),
        ));
        return None;
    };
    let adapter = read_string(
        path,
        &format!("configuration {name}"),
        table,
        "adapter",
        warnings,
    )?;
    let request = read_string(
        path,
        &format!("configuration {name}"),
        table,
        "request",
        warnings,
    )?;
    if request != "launch" && request != "attach" {
        warnings.push(DebugWarning::new(
            path,
            format!("configuration {name}: rejected request: expected launch or attach"),
        ));
        return None;
    }
    let languages = read_strings(
        path,
        &format!("configuration {name}"),
        table,
        "languages",
        warnings,
    );
    let arguments = configuration_arguments(table);
    Some(DebugConfiguration {
        name: name.to_string(),
        adapter,
        languages,
        request,
        arguments,
    })
}

pub(super) fn configuration_arguments(table: &toml::map::Map<String, toml::Value>) -> Value {
    let mut arguments = Map::new();
    for (key, value) in table {
        if CONFIGURATION_FIELDS.contains(&key.as_str()) {
            continue;
        }
        arguments.insert(key.clone(), super::expand::toml_to_json(value.clone()));
    }
    Value::Object(arguments)
}

fn read_string(
    path: &Path,
    owner: &str,
    table: &toml::map::Map<String, toml::Value>,
    field: &str,
    warnings: &mut Vec<DebugWarning>,
) -> Option<String> {
    let Some(value) = table.get(field).and_then(toml::Value::as_str) else {
        warnings.push(DebugWarning::new(
            path,
            format!("{owner}: rejected {field}: expected a non-empty string"),
        ));
        return None;
    };
    let value = value.trim();
    if value.is_empty() {
        warnings.push(DebugWarning::new(
            path,
            format!("{owner}: rejected {field}: expected a non-empty string"),
        ));
        return None;
    }
    Some(value.to_string())
}

fn read_strings(
    path: &Path,
    owner: &str,
    table: &toml::map::Map<String, toml::Value>,
    field: &str,
    warnings: &mut Vec<DebugWarning>,
) -> Vec<String> {
    let Some(value) = table.get(field) else {
        return Vec::new();
    };
    let Some(array) = value.as_array() else {
        warnings.push(DebugWarning::new(
            path,
            format!("{owner}: rejected {field}: expected an array of strings"),
        ));
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|item| {
            let value = item.as_str()?.trim();
            if value.is_empty() {
                warnings.push(DebugWarning::new(
                    path,
                    format!("{owner}: rejected {field}: empty string"),
                ));
                return None;
            }
            Some(value.to_string())
        })
        .collect()
}

fn read_env(
    path: &Path,
    owner: &str,
    table: &toml::map::Map<String, toml::Value>,
    warnings: &mut Vec<DebugWarning>,
) -> BTreeMap<String, String> {
    let Some(value) = table.get("env") else {
        return BTreeMap::new();
    };
    let Some(env) = value.as_table() else {
        warnings.push(DebugWarning::new(
            path,
            format!("{owner}: rejected env: expected a table of strings"),
        ));
        return BTreeMap::new();
    };
    env.iter()
        .filter_map(|(key, value)| {
            let Some(value) = value.as_str() else {
                warnings.push(DebugWarning::new(
                    path,
                    format!("{owner}: rejected env {key}: expected a string"),
                ));
                return None;
            };
            Some((key.clone(), value.to_string()))
        })
        .collect()
}

fn warn_unknown<'a>(
    path: &Path,
    owner: &str,
    keys: impl Iterator<Item = &'a String>,
    allowed: &[&str],
    warnings: &mut Vec<DebugWarning>,
) {
    for key in keys {
        if !allowed.contains(&key.as_str()) {
            warnings.push(DebugWarning::new(
                path,
                format!("{owner}: {key}: ignored unknown field"),
            ));
        }
    }
}
