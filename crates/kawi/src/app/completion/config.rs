use crate::app::key_binding::KeyBinding;

pub(crate) struct CompletionKeys {
    pub(crate) up: KeyBinding,
    pub(crate) down: KeyBinding,
}

impl Default for CompletionKeys {
    fn default() -> Self {
        Self {
            up: KeyBinding::control('b'),
            down: KeyBinding::control('f'),
        }
    }
}

pub(crate) struct CompletionOptions {
    pub(crate) auto: bool,
    pub(crate) signature: bool,
    pub(crate) keys: CompletionKeys,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            auto: false,
            signature: true,
            keys: CompletionKeys::default(),
        }
    }
}
