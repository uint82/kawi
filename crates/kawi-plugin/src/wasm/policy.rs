use std::collections::BTreeSet;

use crate::capability::Capability;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportRule {
    pub prefix: &'static str,
    pub capabilities: &'static [Capability],
}

pub const WASM_IMPORTS: &[ImportRule] = &[
    ImportRule {
        prefix: "kawi:plugin/types@0.8.0",
        capabilities: &[],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-read@0.8.0",
        capabilities: &[Capability::EditorRead],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-write@0.8.0",
        capabilities: &[Capability::EditorWrite],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-buffers@0.8.0",
        capabilities: &[Capability::EditorBuffers],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-buffers-write@0.8.0",
        capabilities: &[Capability::EditorBuffersWrite],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-windows@0.8.0",
        capabilities: &[Capability::EditorWindows],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-control@0.8.0",
        capabilities: &[Capability::EditorControl],
    },
    ImportRule {
        prefix: "kawi:plugin/options-read@0.8.0",
        capabilities: &[Capability::OptionsRead],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-execute@0.8.0",
        capabilities: &[Capability::EditorExecute],
    },
    ImportRule {
        prefix: "kawi:plugin/config-read@0.8.0",
        capabilities: &[],
    },
    ImportRule {
        prefix: "kawi:plugin/ui-message@0.8.0",
        capabilities: &[Capability::UiMessage],
    },
    ImportRule {
        prefix: "kawi:plugin/ui-prompt@0.8.0",
        capabilities: &[Capability::UiPrompt],
    },
    ImportRule {
        prefix: "kawi:plugin/ui-progress@0.8.0",
        capabilities: &[Capability::UiProgress],
    },
    ImportRule {
        prefix: "kawi:plugin/diagnostics-publish@0.8.0",
        capabilities: &[Capability::DiagnosticsPublish],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-decorations@0.8.0",
        capabilities: &[Capability::EditorDecorations],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-signs@0.8.0",
        capabilities: &[Capability::EditorDecorations],
    },
    ImportRule {
        prefix: "kawi:plugin/ui-statusline@0.8.0",
        capabilities: &[Capability::UiStatusline],
    },
    ImportRule {
        prefix: "kawi:plugin/ui-panel@0.8.0",
        capabilities: &[Capability::UiPanel],
    },
    ImportRule {
        prefix: "kawi:plugin/process-run@0.8.0",
        capabilities: &[Capability::ProcessSpawn],
    },
    ImportRule {
        prefix: "kawi:plugin/process-jobs@0.8.0",
        capabilities: &[Capability::ProcessSpawn],
    },
    ImportRule {
        prefix: "wasi:filesystem/",
        capabilities: &[Capability::FsRead, Capability::FsWrite],
    },
    ImportRule {
        prefix: "wasi:sockets/",
        capabilities: &[Capability::NetConnect],
    },
    ImportRule {
        prefix: "wasi:cli/",
        capabilities: &[],
    },
    ImportRule {
        prefix: "wasi:io/",
        capabilities: &[],
    },
    ImportRule {
        prefix: "wasi:clocks/",
        capabilities: &[],
    },
    ImportRule {
        prefix: "wasi:random/",
        capabilities: &[],
    },
];

pub fn import_rule(name: &str) -> Option<&'static ImportRule> {
    WASM_IMPORTS
        .iter()
        .find(|rule| name.starts_with(rule.prefix))
}

pub fn import_capabilities(name: &str) -> Option<&'static [Capability]> {
    import_rule(name).map(|rule| rule.capabilities)
}

pub fn requested_import_capabilities<'a>(
    imports: impl IntoIterator<Item = &'a str>,
) -> BTreeSet<Capability> {
    imports
        .into_iter()
        .filter_map(import_capabilities)
        .flatten()
        .copied()
        .collect()
}

pub fn missing_import_capabilities<'a>(
    imports: impl IntoIterator<Item = &'a str>,
    requested: &BTreeSet<Capability>,
    granted: &BTreeSet<Capability>,
) -> Vec<Capability> {
    let mut missing = BTreeSet::new();
    for import in imports {
        let Some(capabilities) = import_capabilities(import) else {
            continue;
        };
        if capabilities
            .iter()
            .any(|capability| requested.contains(capability) && granted.contains(capability))
        {
            continue;
        }
        if let Some(capability) = capabilities
            .iter()
            .find(|capability| requested.contains(capability) && !granted.contains(capability))
        {
            missing.insert(*capability);
        }
    }
    missing.into_iter().collect()
}

pub fn unauthorized_imports<'a>(
    imports: impl IntoIterator<Item = &'a str>,
    requested: &BTreeSet<Capability>,
) -> Vec<&'a str> {
    imports
        .into_iter()
        .filter(|name| match import_capabilities(name) {
            None => true,
            Some(capabilities) => capabilities
                .iter()
                .any(|capability| !requested.contains(capability)),
        })
        .collect()
}

#[cfg(test)]
mod tests;
