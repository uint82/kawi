use std::collections::VecDeque;
use std::time::{Duration, Instant};

use kawi_plugin::{Capability, PluginError, RpcError, CAPABILITY_DENIED};
use serde_json::Value;

use super::runtime::MESSAGE_THROTTLE;
use super::PluginRuntime;

pub(super) const MAX_QUEUED_INVOCATIONS: usize = 4;
pub(super) const MAX_QUEUE_WAIT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub(crate) struct QueuedInvocation {
    pub(super) plugin: String,
    pub(super) command: String,
    pub(super) args: Value,
    pub(super) enqueued: Instant,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct QueueCounters {
    pub(super) queued: u64,
    pub(super) rejected: u64,
    pub(super) dropped: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct QueueDrop {
    pub(super) plugin: String,
    pub(super) command: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QueueOutcome {
    Start,
    Queued { depth: usize },
    Coalesced { depth: usize },
}

impl PluginRuntime {
    pub(crate) fn enqueue_invocation(
        &mut self,
        target: u32,
        args: Value,
    ) -> Result<QueueOutcome, PluginError> {
        let (plugin, command, coalesce) = self.resolved_target(target)?;
        self.ensure_command_allowed(&plugin, &command)?;
        self.ensure_invokable(&plugin)?;
        if !self.in_flight.contains(&plugin) {
            self.in_flight.insert(plugin);
            return Ok(QueueOutcome::Start);
        }
        let depth = self.queues.get(&plugin).map_or(0, VecDeque::len);
        self.ensure_queue_capacity(&plugin, depth)?;
        if coalesce && self.has_duplicate(&plugin, &command, &args) {
            return Ok(QueueOutcome::Coalesced { depth });
        }
        self.push_queued(plugin, command, args);
        Ok(QueueOutcome::Queued {
            depth: depth.saturating_add(1),
        })
    }

    fn resolved_target(&self, target: u32) -> Result<(String, String, bool), PluginError> {
        self.targets
            .get(target as usize)
            .map(|target| {
                (
                    target.plugin.clone(),
                    target.command.clone(),
                    target.coalesce,
                )
            })
            .ok_or_else(|| PluginError::Spawn("unknown plugin command".to_string()))
    }

    fn ensure_queue_capacity(&mut self, plugin: &str, depth: usize) -> Result<(), PluginError> {
        if depth < MAX_QUEUED_INVOCATIONS {
            return Ok(());
        }
        self.queue_stats
            .entry(plugin.to_string())
            .or_default()
            .rejected += 1;
        Err(PluginError::QueueFull)
    }

    fn has_duplicate(&self, plugin: &str, command: &str, args: &Value) -> bool {
        self.queues.get(plugin).is_some_and(|queue| {
            queue
                .iter()
                .any(|queued| queued.command == command && &queued.args == args)
        })
    }

    fn push_queued(&mut self, plugin: String, command: String, args: Value) {
        self.queues
            .entry(plugin.clone())
            .or_default()
            .push_back(QueuedInvocation {
                plugin: plugin.clone(),
                command,
                args,
                enqueued: Instant::now(),
            });
        self.queue_stats.entry(plugin).or_default().queued += 1;
    }

    pub(crate) fn finish_invocation(
        &mut self,
        plugin: &str,
        result: &Result<Value, PluginError>,
    ) -> Vec<QueueDrop> {
        self.in_flight.remove(plugin);
        if is_transport_failure(result) {
            self.drop_queues(Some(plugin))
        } else {
            Vec::new()
        }
    }

    pub(crate) fn abandon_invocation(&mut self, plugin: &str) {
        self.in_flight.remove(plugin);
    }

    pub(crate) fn take_queued(&mut self, plugin: &str) -> Option<QueuedInvocation> {
        let queued = self.queues.get_mut(plugin)?.pop_front()?;
        if self.queues.get(plugin).is_some_and(VecDeque::is_empty) {
            self.queues.remove(plugin);
        }
        self.in_flight.insert(plugin.to_string());
        Some(queued)
    }

    pub(crate) fn expire_queued(&mut self) -> Vec<QueueDrop> {
        let now = Instant::now();
        let wait = self.queue_wait_override.unwrap_or(MAX_QUEUE_WAIT);
        let mut dropped = Vec::new();
        let mut counts = Vec::new();
        for (plugin, queue) in &mut self.queues {
            let before = queue.len();
            queue.retain(|queued| {
                let keep = now.duration_since(queued.enqueued) < wait;
                if !keep {
                    dropped.push(QueueDrop {
                        plugin: plugin.clone(),
                        command: queued.command.clone(),
                    });
                }
                keep
            });
            let count = before - queue.len();
            if count > 0 {
                counts.push((plugin.clone(), count as u64));
            }
        }
        self.queues.retain(|_, queue| !queue.is_empty());
        for (plugin, count) in counts {
            self.queue_stats.entry(plugin).or_default().dropped += count;
        }
        dropped
    }

    pub(crate) fn drop_queues(&mut self, plugin: Option<&str>) -> Vec<QueueDrop> {
        let plugins = match plugin {
            Some(plugin) => vec![plugin.to_string()],
            None => self.queues.keys().cloned().collect(),
        };
        let mut dropped = Vec::new();
        for plugin in plugins {
            self.in_flight.remove(&plugin);
            let Some(mut queue) = self.queues.remove(&plugin) else {
                continue;
            };
            let count = queue.len() as u64;
            dropped.extend(queue.drain(..).map(|queued| QueueDrop {
                plugin: plugin.clone(),
                command: queued.command,
            }));
            self.queue_stats.entry(plugin).or_default().dropped += count;
        }
        dropped
    }

    pub(crate) fn queue_depth(&self, plugin: &str) -> usize {
        self.queues.get(plugin).map_or(0, VecDeque::len)
    }

    pub(crate) fn queue_counters(&self, plugin: &str) -> QueueCounters {
        self.queue_stats.get(plugin).copied().unwrap_or_default()
    }

    pub(crate) fn command_denied(&self, plugin: &str, command: &str) -> Option<Capability> {
        let target = self
            .targets
            .iter()
            .find(|target| target.plugin == plugin && target.command == command)?;
        target
            .required
            .iter()
            .copied()
            .find(|capability| !self.has_capability(plugin, *capability))
    }

    pub(crate) fn command_error(&self, plugin: &str, capability: Capability) -> PluginError {
        PluginError::Rpc(self.command_rpc_error(plugin, capability))
    }

    pub(crate) fn command_rpc_error(&self, plugin: &str, capability: Capability) -> RpcError {
        RpcError::capability_denied(
            capability.name(),
            self.request_scope(plugin, capability)
                .filter(|scope| !matches!(scope, kawi_plugin::Scope::Unscoped))
                .map(|scope| serde_json::Value::Object(scope.to_json_object())),
        )
    }

    pub(crate) fn allow_busy_message(&mut self, plugin: &str) -> bool {
        let now = Instant::now();
        if self
            .busy_messages
            .get(plugin)
            .is_some_and(|last| now.duration_since(*last) < MESSAGE_THROTTLE)
        {
            return false;
        }
        self.busy_messages.insert(plugin.to_string(), now);
        true
    }

    fn ensure_command_allowed(&self, plugin: &str, command: &str) -> Result<(), PluginError> {
        match self.command_denied(plugin, command) {
            Some(capability) => Err(self.command_error(plugin, capability)),
            None => Ok(()),
        }
    }

    fn ensure_invokable(&self, plugin: &str) -> Result<(), PluginError> {
        match self.status(plugin) {
            Some(kawi_plugin::PluginStatus::Backoff { remaining_ms }) => {
                Err(PluginError::Backoff { remaining_ms })
            }
            Some(kawi_plugin::PluginStatus::Disabled) => Err(PluginError::Disabled),
            _ => Ok(()),
        }
    }
}

fn is_transport_failure(result: &Result<Value, PluginError>) -> bool {
    matches!(
        result,
        Err(PluginError::Spawn(_))
            | Err(PluginError::Timeout)
            | Err(PluginError::ProcessExited)
            | Err(PluginError::Trap(_))
            | Err(PluginError::Protocol(_))
            | Err(PluginError::Initialize(_))
    )
}

pub(super) fn capability_from_error(error: &PluginError) -> Option<Capability> {
    let PluginError::Rpc(error) = error else {
        return None;
    };
    if error.code != CAPABILITY_DENIED {
        return None;
    }
    error
        .data
        .as_ref()?
        .get("capability")?
        .as_str()
        .and_then(Capability::parse)
}
