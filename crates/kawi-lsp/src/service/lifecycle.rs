use super::*;
use crate::workspace;
use std::collections::BTreeSet;

impl Manager {
    pub(super) fn handle_lifecycle(
        &mut self,
        action: LifecycleAction,
        path: Option<&Path>,
        server: Option<&str>,
    ) {
        let targets = self.lifecycle_targets(path, server);
        if targets.is_empty() {
            self.emit_lifecycle_message(action, path, server);
            return;
        }
        match action {
            LifecycleAction::Start => self.start_targets(targets),
            LifecycleAction::Stop => self.stop_targets(targets),
            LifecycleAction::Restart => self.restart_targets(targets),
        }
    }

    fn lifecycle_targets(&self, path: Option<&Path>, server: Option<&str>) -> Vec<ClientKey> {
        if let Some(server) = server {
            let mut targets: BTreeSet<ClientKey> = self.named_targets(server).into_iter().collect();
            if let Some(path) = path {
                targets.extend(self.document_targets(path, Some(server)));
            }
            return targets.into_iter().collect();
        }
        path.map(|path| self.document_targets(path, None))
            .unwrap_or_default()
    }

    fn document_targets(&self, path: &Path, server: Option<&str>) -> Vec<ClientKey> {
        let path = workspace::absolute(path);
        let Some(language) = self.languages.language_for_path(&path) else {
            return Vec::new();
        };
        let root = workspace::root_for(&path, &language.root_markers);
        language
            .servers
            .iter()
            .filter(|name| server.is_none_or(|target| target == name.as_str()))
            .filter(|name| {
                self.languages
                    .server(name)
                    .is_some_and(|config| config.enabled)
            })
            .map(|name| ClientKey {
                server: name.clone(),
                root: root.clone(),
            })
            .collect()
    }

    fn named_targets(&self, server: &str) -> BTreeSet<ClientKey> {
        let mut targets: BTreeSet<ClientKey> = self
            .stopped
            .iter()
            .filter(|key| key.server == server)
            .cloned()
            .collect();
        targets.extend(
            self.clients
                .keys()
                .filter(|key| key.server == server)
                .cloned(),
        );
        targets
    }

    fn start_targets(&mut self, targets: Vec<ClientKey>) {
        for key in targets {
            self.start_key(key);
        }
    }

    fn start_key(&mut self, key: ClientKey) {
        self.stopped.remove(&key);
        if self.clients.contains_key(&key) {
            return;
        }
        let Some(config) = self.languages.server(&key.server).cloned() else {
            return;
        };
        if !config.enabled {
            return;
        }
        if !self.languages.server_available(&key.server) {
            self.emit_status(&key.server, ServerState::Missing, None);
            return;
        }
        self.attach_document(&key);
        self.start_client(key.clone(), &config, &key.root);
    }

    fn stop_targets(&mut self, targets: Vec<ClientKey>) {
        for key in targets {
            self.stop_key(key);
        }
    }

    fn stop_key(&mut self, key: ClientKey) {
        self.stopped.insert(key.clone());
        if self.clients.contains_key(&key) {
            self.shutdown_running_client(&key, "language server stopped");
        } else {
            self.emit_status(&key.server, ServerState::Stopped, None);
        }
    }

    fn restart_targets(&mut self, targets: Vec<ClientKey>) {
        for key in targets {
            self.stopped.remove(&key);
            if self.clients.contains_key(&key) {
                self.shutdown_running_client(&key, "language server restarted");
            }
            self.start_key(key);
        }
    }

    fn shutdown_running_client(&mut self, key: &ClientKey, message: &str) {
        let Some(mut client) = self.clients.remove(key) else {
            return;
        };
        self.shutdown_client(key, &mut client);
        client.client.terminate();
        self.cleanup_client(key, ServerState::Stopped, Some(message.to_string()), false);
    }

    fn attach_document(&mut self, key: &ClientKey) {
        let paths: Vec<PathBuf> = self.documents.keys().cloned().collect();
        for path in paths {
            let Some(language) = self.languages.language_for_path(&path) else {
                continue;
            };
            if !language.servers.contains(&key.server) {
                continue;
            }
            if workspace::root_for(&path, &language.root_markers) != key.root {
                continue;
            }
            if let Some(document) = self.documents.get_mut(&path) {
                if !document.clients.contains(key) {
                    document.clients.push(key.clone());
                }
            }
        }
    }

    fn emit_lifecycle_message(
        &self,
        action: LifecycleAction,
        path: Option<&Path>,
        server: Option<&str>,
    ) {
        let target = match (path, server) {
            (_, Some(server)) => format!(" for {server}"),
            (Some(path), None) => format!(" for {}", path.display()),
            (None, None) => String::new(),
        };
        self.emit(LspEvent::Message {
            server: "lsp".to_string(),
            level: MessageLevel::Info,
            message: format!("no language servers to {}{target}", action.verb()),
        });
    }
}
