use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use super::{Capability, EffectiveCapabilities, Scope};

#[derive(Clone, Default)]
pub struct CapabilityRegistry {
    grants: Arc<RwLock<BTreeMap<String, EffectiveCapabilities>>>,
}

impl CapabilityRegistry {
    pub fn replace(&self, grants: BTreeMap<String, EffectiveCapabilities>) {
        let mut current = self
            .grants
            .write()
            .unwrap_or_else(|error| error.into_inner());
        *current = grants;
    }

    pub fn get(&self, plugin: &str) -> EffectiveCapabilities {
        self.grants
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .get(plugin)
            .cloned()
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> BTreeMap<String, EffectiveCapabilities> {
        self.grants
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn has(&self, plugin: &str, capability: Capability) -> bool {
        self.grants
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .get(plugin)
            .is_some_and(|effective| effective.has(capability))
    }

    pub fn scope(&self, plugin: &str, capability: Capability) -> Option<Scope> {
        self.grants
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .get(plugin)
            .and_then(|effective| effective.scope(capability))
            .cloned()
    }
}
