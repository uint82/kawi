use std::collections::BTreeSet;

use crate::capability::Capability;

pub const KAWI_PACKAGE_PREFIX: &str = "kawi:plugin/";
pub const WASI_PACKAGE_PREFIX: &str = "wasi:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportRule {
    pub prefix: &'static str,
    pub capabilities: &'static [Capability],
}

pub const WASM_IMPORTS: &[ImportRule] = &[
    ImportRule {
        prefix: "kawi:plugin/editor-read@0.1.0",
        capabilities: &[Capability::EditorRead],
    },
    ImportRule {
        prefix: "kawi:plugin/editor-write@0.1.0",
        capabilities: &[Capability::EditorWrite],
    },
    ImportRule {
        prefix: "kawi:plugin/ui-message@0.1.0",
        capabilities: &[Capability::UiMessage],
    },
    ImportRule {
        prefix: "kawi:plugin/diagnostics-publish@0.1.0",
        capabilities: &[Capability::DiagnosticsPublish],
    },
    ImportRule {
        prefix: "kawi:plugin/process-run@0.1.0",
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

pub fn unauthorized_imports<'a>(
    imports: impl IntoIterator<Item = &'a str>,
    requested: &BTreeSet<Capability>,
) -> Vec<&'a str> {
    imports
        .into_iter()
        .filter(|name| {
            let Some(capabilities) = import_capabilities(name) else {
                return name.starts_with(KAWI_PACKAGE_PREFIX)
                    || name.starts_with(WASI_PACKAGE_PREFIX);
            };
            !capabilities.is_empty()
                && !capabilities
                    .iter()
                    .any(|capability| requested.contains(capability))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_interfaces_to_the_same_capability_names() {
        assert_eq!(
            import_capabilities("kawi:plugin/editor-read@0.1.0"),
            Some(&[Capability::EditorRead][..])
        );
        assert_eq!(
            import_capabilities("wasi:sockets/tcp@0.2.12"),
            Some(&[Capability::NetConnect][..])
        );
        assert_eq!(
            import_capabilities("wasi:filesystem/types@0.2.12"),
            Some(&[Capability::FsRead, Capability::FsWrite][..])
        );
        assert_eq!(
            import_capabilities("wasi:cli/environment@0.2.12"),
            Some(&[][..])
        );
    }

    #[test]
    fn rejects_imports_beyond_manifest_requests() {
        let requested = BTreeSet::from([Capability::EditorRead, Capability::FsRead]);
        let imports = [
            "wasi:cli/environment@0.2.12",
            "wasi:filesystem/types@0.2.12",
            "kawi:plugin/editor-read@0.1.0",
            "kawi:plugin/editor-write@0.1.0",
            "wasi:sockets/tcp@0.2.12",
        ];
        assert_eq!(
            unauthorized_imports(imports.into_iter(), &requested),
            vec!["kawi:plugin/editor-write@0.1.0", "wasi:sockets/tcp@0.2.12"]
        );
    }

    #[test]
    fn rejects_unknown_kawi_and_wasi_interfaces() {
        let requested = BTreeSet::from(Capability::ALL);
        assert_eq!(
            unauthorized_imports(
                ["kawi:plugin/unknown@0.1.0", "wasi:unknown/interface@0.2.12",].into_iter(),
                &requested
            ),
            vec!["kawi:plugin/unknown@0.1.0", "wasi:unknown/interface@0.2.12"]
        );
    }

    #[test]
    fn wit_package_matches_the_import_table() {
        let wit = include_str!("../../wit/plugin.wit");
        assert!(wit.contains("package kawi:plugin@0.1.0;"));
        for rule in WASM_IMPORTS
            .iter()
            .filter(|rule| rule.prefix.starts_with(KAWI_PACKAGE_PREFIX))
        {
            let name = rule
                .prefix
                .trim_start_matches(KAWI_PACKAGE_PREFIX)
                .split('@')
                .next()
                .unwrap();
            assert!(wit.contains(&format!("interface {name} {{")), "{name}");
        }
        assert!(wit.contains("export lifecycle;"));
    }

    #[test]
    fn host_interfaces_share_the_process_capability_table() {
        for rule in WASM_IMPORTS
            .iter()
            .filter(|rule| rule.prefix.starts_with(KAWI_PACKAGE_PREFIX))
        {
            for capability in rule.capabilities {
                if *capability == Capability::ProcessSpawn {
                    continue;
                }
                assert!(
                    crate::protocol::HOST_METHODS
                        .iter()
                        .any(|method| crate::capability::host_method_capability(method)
                            == crate::capability::MethodCapability::Required(*capability)),
                    "{}",
                    capability.name()
                );
            }
        }
    }

    #[test]
    fn records_capabilities_used_by_imports() {
        assert_eq!(
            requested_import_capabilities(
                [
                    "kawi:plugin/editor-read@0.1.0",
                    "wasi:filesystem/types@0.2.12",
                ]
                .into_iter()
            ),
            BTreeSet::from([
                Capability::EditorRead,
                Capability::FsRead,
                Capability::FsWrite
            ])
        );
    }
}
