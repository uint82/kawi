use std::time::{Duration, Instant};

use super::binding::{Binding, BindingSource, KeyContext, ResolvedCommand};
use super::chord::{KeyChord, KeySequence};

pub(crate) const DEFAULT_MAP_TIMEOUT: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolveOutcome {
    Passthrough,
    Pending,
    Command(ResolvedCommand),
    Fallback(Vec<KeyChord>),
}

pub(crate) struct KeymapResolver {
    bindings: Vec<Binding>,
    pending: Vec<KeyChord>,
    pending_context: Option<KeyContext>,
    pending_since: Option<Instant>,
    timeout: Duration,
}

impl KeymapResolver {
    pub(crate) fn new() -> Self {
        Self {
            bindings: Vec::new(),
            pending: Vec::new(),
            pending_context: None,
            pending_since: None,
            timeout: DEFAULT_MAP_TIMEOUT,
        }
    }

    pub(crate) fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    pub(crate) fn set_bindings(&mut self, bindings: Vec<Binding>) {
        self.bindings = bindings;
        self.clear_pending();
    }

    pub(crate) fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub(crate) fn bind(&mut self, binding: Binding) {
        if let Some(index) = self
            .bindings
            .iter()
            .position(|item| item.mode == binding.mode && item.keys == binding.keys)
        {
            if self.bindings[index].source.priority() > binding.source.priority() {
                return;
            }
            self.bindings[index] = binding;
        } else {
            self.bindings.push(binding);
        }
        self.clear_pending();
    }

    pub(crate) fn unbind(&mut self, mode: KeyContext, keys: &KeySequence) {
        self.bindings
            .retain(|binding| binding.mode != mode || binding.keys != *keys);
        self.clear_pending();
    }

    pub(crate) fn unbind_source(
        &mut self,
        mode: KeyContext,
        keys: &KeySequence,
        source: BindingSource,
    ) {
        self.bindings.retain(|binding| {
            binding.mode != mode || binding.keys != *keys || binding.source != source
        });
        self.clear_pending();
    }

    pub(crate) fn feed(
        &mut self,
        key: KeyChord,
        context: KeyContext,
        now: Instant,
    ) -> ResolveOutcome {
        if self.pending_context != Some(context) {
            self.clear_pending();
        }
        let mut candidate = self.pending.clone();
        candidate.push(key);
        if !self.has_prefix(context, &candidate) {
            if self.pending.is_empty() {
                return ResolveOutcome::Passthrough;
            }
            let keys = std::mem::take(&mut self.pending);
            self.clear_pending();
            return ResolveOutcome::Fallback(keys);
        }
        self.pending_context = Some(context);
        let exact = self.exact_binding(context, &candidate).cloned();
        let has_longer = self.has_longer_prefix(context, &candidate);
        if let Some(binding) = exact {
            if has_longer {
                self.pending = candidate;
                self.pending_since = Some(now);
                return ResolveOutcome::Pending;
            }
            self.clear_pending();
            return ResolveOutcome::Command(binding.into());
        }
        self.pending = candidate;
        self.pending_since = Some(now);
        ResolveOutcome::Pending
    }

    pub(crate) fn take_timeout(&mut self, now: Instant) -> Option<ResolveOutcome> {
        let since = self.pending_since?;
        if now.duration_since(since) < self.timeout {
            return None;
        }
        let context = self.pending_context?;
        let exact = self.exact_binding(context, &self.pending).cloned();
        if let Some(binding) = exact {
            self.clear_pending();
            return Some(ResolveOutcome::Command(binding.into()));
        }
        let keys = std::mem::take(&mut self.pending);
        self.clear_pending();
        Some(ResolveOutcome::Fallback(keys))
    }

    pub(crate) fn timeout_in(&self, now: Instant) -> Option<Duration> {
        let since = self.pending_since?;
        Some(self.timeout.saturating_sub(now.duration_since(since)))
    }

    pub(crate) fn pending_text(&self) -> Option<String> {
        (!self.pending.is_empty())
            .then(|| KeySequence::from_chords(self.pending.clone()).to_string())
    }

    pub(crate) fn has_pending(&self) -> bool {
        self.pending_context.is_some()
    }

    pub(crate) fn clear_pending(&mut self) {
        self.pending.clear();
        self.pending_context = None;
        self.pending_since = None;
    }

    pub(crate) fn pending_context(&self) -> Option<KeyContext> {
        self.pending_context
    }

    pub(crate) fn has_user_binding(&self, mode: KeyContext, keys: &[KeyChord]) -> bool {
        self.bindings
            .iter()
            .any(|binding| binding.source == BindingSource::User && binding.matches(mode, keys))
    }

    fn exact_binding(&self, context: KeyContext, keys: &[KeyChord]) -> Option<&Binding> {
        self.bindings
            .iter()
            .rev()
            .find(|binding| binding.matches(context, keys))
    }

    fn has_prefix(&self, context: KeyContext, keys: &[KeyChord]) -> bool {
        self.bindings
            .iter()
            .any(|binding| binding.mode == context && binding.keys.starts_with(keys))
    }

    fn has_longer_prefix(&self, context: KeyContext, keys: &[KeyChord]) -> bool {
        self.bindings.iter().any(|binding| {
            binding.mode == context
                && binding.keys.len() > keys.len()
                && binding.keys.starts_with(keys)
        })
    }
}

impl Default for KeymapResolver {
    fn default() -> Self {
        Self::new()
    }
}
