use std::collections::HashMap;

use kawi_editor::Action;

mod builtin;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CommandId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandHandler {
    Action(Action),
    SendKeys,
    Noop,
    Plugin(u32),
    PluginCancel,
    PluginPanel,
    LspStatus,
    LspStart,
    LspStop,
    LspRestart,
    Debug(DebugBuiltin),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugBuiltin {
    Start,
    Restart,
    Stop,
    Terminate,
    Panel,
    Continue,
    Next,
    StepIn,
    StepOut,
    ToggleBreakpoint,
    Stack,
    Variables,
    Threads,
    Functions,
    Exceptions,
    Console,
    Watches,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandSource {
    Builtin,
    Plugin(String),
}

impl CommandSource {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Builtin => String::new(),
            Self::Plugin(name) => format!(" [plugin: {name}]"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CommandSpec {
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) description: String,
    pub(crate) handler: CommandHandler,
    pub(crate) source: CommandSource,
}

impl CommandSpec {
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
}

pub(crate) struct CommandRegistry {
    specs: Vec<Option<CommandSpec>>,
    index: HashMap<String, CommandId>,
    retired: HashMap<String, CommandId>,
}

impl CommandRegistry {
    pub(crate) fn new() -> Self {
        let mut specs = Vec::new();
        let mut index = HashMap::new();
        for builtin in builtin::all() {
            let id = CommandId(specs.len() as u32);
            insert_name(&mut index, builtin.name, id);
            for alias in builtin.aliases {
                insert_name(&mut index, alias, id);
            }
            specs.push(Some(CommandSpec {
                name: builtin.name.to_string(),
                aliases: builtin
                    .aliases
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                description: builtin.description.to_string(),
                handler: builtin.handler,
                source: CommandSource::Builtin,
            }));
        }
        Self {
            specs,
            index,
            retired: HashMap::new(),
        }
    }

    pub(crate) fn find(&self, name: &str) -> Option<CommandId> {
        self.index.get(name.trim()).copied()
    }

    pub(crate) fn spec(&self, id: CommandId) -> Option<&CommandSpec> {
        self.specs.get(id.0 as usize).and_then(Option::as_ref)
    }

    pub(crate) fn specs(&self) -> impl Iterator<Item = &CommandSpec> {
        self.specs.iter().filter_map(Option::as_ref)
    }

    pub(crate) fn register_plugin(
        &mut self,
        registration: PluginCommandRegistration,
    ) -> Result<CommandId, String> {
        for name in registration.names() {
            if self.index.contains_key(name) {
                return Err(format!("command name already exists: {name}"));
            }
        }
        let spec = CommandSpec {
            name: registration.name.clone(),
            aliases: registration.aliases,
            description: registration.description,
            handler: CommandHandler::Plugin(registration.target),
            source: CommandSource::Plugin(registration.plugin),
        };
        let id = if let Some(id) = self.retired.remove(&spec.name) {
            self.specs[id.0 as usize] = Some(spec.clone());
            id
        } else {
            let id = CommandId(self.specs.len() as u32);
            self.specs.push(Some(spec.clone()));
            id
        };
        for name in spec.names() {
            self.index.insert(name.to_string(), id);
        }
        Ok(id)
    }

    pub(crate) fn unregister_plugin(
        &mut self,
        name: &str,
        plugin: &str,
    ) -> Result<CommandId, String> {
        let id = *self
            .index
            .get(name)
            .ok_or_else(|| format!("unknown command: {name}"))?;
        let spec = self
            .specs
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| format!("unknown command: {name}"))?;
        if spec.name != name {
            return Err(format!("command alias cannot be unregistered: {name}"));
        }
        if !matches!(&spec.source, CommandSource::Plugin(owner) if owner == plugin) {
            return Err(format!("command is not owned by plugin: {name}"));
        }
        let spec = spec.clone();
        self.retire(id, spec);
        Ok(id)
    }

    pub(crate) fn remove_plugin(&mut self, plugin: &str) {
        let removed = self
            .specs
            .iter()
            .enumerate()
            .filter_map(|(index, spec)| {
                matches!(spec.as_ref()?.source, CommandSource::Plugin(ref name) if name == plugin)
                    .then_some(CommandId(index as u32))
            })
            .collect::<Vec<_>>();
        for id in removed {
            if let Some(spec) = self.specs[id.0 as usize].take() {
                self.retire_names(id, &spec);
            }
        }
    }

    fn retire(&mut self, id: CommandId, spec: CommandSpec) {
        self.specs[id.0 as usize] = None;
        self.retire_names(id, &spec);
    }

    fn retire_names(&mut self, id: CommandId, spec: &CommandSpec) {
        for name in spec.names() {
            self.index.remove(name);
        }
        self.retired.insert(spec.name.clone(), id);
    }
}

pub(crate) struct PluginCommandRegistration {
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) description: String,
    pub(crate) plugin: String,
    pub(crate) target: u32,
}

impl PluginCommandRegistration {
    fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
}

fn insert_name(index: &mut HashMap<String, CommandId>, name: &str, id: CommandId) {
    assert!(
        index.insert(name.to_string(), id).is_none(),
        "duplicate command name: {name}"
    );
}
