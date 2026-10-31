use std::path::{Path, PathBuf};

use kawi_plugin::PluginLock;

use crate::state::StatePaths;

mod github;
mod http;
mod plan;
mod prompt;
mod source;

#[cfg(test)]
pub(crate) mod test_client;
#[cfg(test)]
mod tests;

pub(crate) use github::{GithubClient, ReleaseClient};
pub(crate) use plan::{
    manifest_requests, prepare, remove_installed, update, PreparedPlugin, PreparedUpdate,
};
pub(crate) use prompt::{PluginPrompt, PromptAction};
pub(crate) use source::PluginSpec;

const SHORT_COMMIT_LEN: usize = 12;

pub(crate) fn short_commit(commit: &str) -> &str {
    commit.get(..SHORT_COMMIT_LEN).unwrap_or(commit)
}

pub(crate) struct PluginManager {
    installed: PathBuf,
    dev: PathBuf,
    lock_path: PathBuf,
    lock: PluginLock,
    warning: Option<String>,
    client: Box<dyn ReleaseClient>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self {
            installed: PathBuf::new(),
            dev: PathBuf::new(),
            lock_path: PathBuf::new(),
            lock: PluginLock::default(),
            warning: None,
            client: Box::new(GithubClient::new()),
        }
    }
}

impl PluginManager {
    pub(crate) fn configure(&mut self, paths: &StatePaths) {
        self.set_paths(
            paths.installed_plugins.clone(),
            paths.plugins.clone(),
            paths.plugin_lock.clone(),
        );
    }

    fn set_paths(&mut self, installed: PathBuf, dev: PathBuf, lock_path: PathBuf) {
        self.installed = installed;
        self.dev = dev;
        self.lock_path = lock_path;
        match PluginLock::load(&self.lock_path) {
            Ok(lock) => {
                self.lock = lock;
                self.warning = None;
            }
            Err(error) => {
                self.lock = PluginLock::default();
                self.warning = Some(error);
            }
        }
    }

    pub(crate) fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn set_client(&mut self, client: Box<dyn ReleaseClient>) {
        self.client = client;
    }

    #[cfg(test)]
    pub(crate) fn configure_for_test(
        &mut self,
        installed: PathBuf,
        dev: PathBuf,
        lock_path: PathBuf,
    ) {
        self.set_paths(installed, dev, lock_path);
    }

    pub(crate) fn dev_dir(&self) -> &Path {
        &self.dev
    }

    pub(crate) fn lock(&self) -> &PluginLock {
        &self.lock
    }

    pub(crate) fn is_managed(&self, name: &str) -> bool {
        self.lock.plugins.contains_key(name)
    }

    pub(crate) fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<(), String> {
        self.ensure_ready()?;
        let previous = self.lock.plugins.get(name).cloned();
        if !self.lock.set_enabled(name, enabled) {
            return Err(format!("plugin {name} is not installed"));
        }
        if let Err(error) = self.lock.save(&self.lock_path) {
            if let Some(previous) = previous {
                self.lock.plugins.insert(name.to_string(), previous);
            }
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn remove(&mut self, name: &str) -> Result<(), String> {
        self.ensure_ready()?;
        let previous = self
            .lock
            .remove(name)
            .ok_or_else(|| format!("plugin {name} is not installed"))?;
        if let Err(error) = self.lock.save(&self.lock_path) {
            self.lock.plugins.insert(name.to_string(), previous);
            return Err(error);
        }
        remove_installed(name, &self.installed)
    }

    pub(crate) fn prepare_install(&self, spec: &PluginSpec) -> Result<PreparedPlugin, String> {
        self.ensure_ready()?;
        let prepared = prepare(spec, self.client.as_ref(), &self.installed)?;
        self.check_source(&prepared)?;
        Ok(prepared)
    }

    pub(crate) fn prepare_update(&self, name: &str) -> Result<PreparedUpdate, String> {
        self.ensure_ready()?;
        let locked = self
            .lock
            .plugins
            .get(name)
            .cloned()
            .ok_or_else(|| format!("plugin {name} is not installed"))?;
        let mut spec = PluginSpec::parse(&locked.source)?;
        spec.reference = locked.reference.clone();
        let previous = manifest_requests(&self.installed, name)
            .ok_or_else(|| format!("installed plugin {name} is missing"))?;
        let prepared = prepare(&spec, self.client.as_ref(), &self.installed)?;
        if prepared.name != name {
            return Err(format!(
                "plugin source now provides {} instead of {name}",
                prepared.name
            ));
        }
        let current = prepared.commit == locked.commit && prepared.sha256 == locked.sha256;
        let mut update = update(prepared, &previous);
        update.current = current;
        Ok(update)
    }

    pub(crate) fn apply_install(&mut self, prepared: &PreparedPlugin) -> Result<(), String> {
        self.ensure_ready()?;
        let name = prepared.name.clone();
        let previous = self.lock.plugins.get(&name).cloned();
        let swap = plan::apply(prepared)?;
        self.lock
            .plugins
            .insert(name.clone(), prepared.lock.clone());
        if let Err(error) = self.lock.save(&self.lock_path) {
            match previous {
                Some(previous) => {
                    self.lock.plugins.insert(name, previous);
                }
                None => {
                    self.lock.remove(&name);
                }
            }
            swap.rollback();
            return Err(error);
        }
        swap.commit()
    }

    pub(crate) fn apply_update(&mut self, update: &PreparedUpdate) -> Result<(), String> {
        self.apply_install(&update.plugin)
    }

    fn ensure_ready(&self) -> Result<(), String> {
        match &self.warning {
            Some(warning) => Err(format!("cannot modify plugins: {warning}")),
            None => Ok(()),
        }
    }

    fn check_source(&self, prepared: &PreparedPlugin) -> Result<(), String> {
        if let Some(locked) = self.lock.plugins.get(&prepared.name) {
            return Err(format!(
                "{} is already installed from {}; use :plugin update",
                prepared.name, locked.source
            ));
        }
        if prepared.destination.exists() {
            return Err(format!(
                "{} exists but is not managed by {}",
                prepared.name,
                self.lock_path.display()
            ));
        }
        Ok(())
    }
}
