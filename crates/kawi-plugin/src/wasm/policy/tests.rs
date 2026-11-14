use super::*;

#[test]
fn maps_interfaces_to_the_same_capability_names() {
    assert_eq!(
        import_capabilities("kawi:plugin/editor-read@0.8.0"),
        Some(&[Capability::EditorRead][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/editor-buffers@0.8.0"),
        Some(&[Capability::EditorBuffers][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/editor-buffers-write@0.8.0"),
        Some(&[Capability::EditorBuffersWrite][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/editor-windows@0.8.0"),
        Some(&[Capability::EditorWindows][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/editor-control@0.8.0"),
        Some(&[Capability::EditorControl][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/options-read@0.8.0"),
        Some(&[Capability::OptionsRead][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/editor-execute@0.8.0"),
        Some(&[Capability::EditorExecute][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/ui-prompt@0.8.0"),
        Some(&[Capability::UiPrompt][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/ui-progress@0.8.0"),
        Some(&[Capability::UiProgress][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/ui-panel@0.8.0"),
        Some(&[Capability::UiPanel][..])
    );
    assert_eq!(
        import_capabilities("kawi:plugin/process-jobs@0.8.0"),
        Some(&[Capability::ProcessSpawn][..])
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
fn requests_one_missing_alternative_per_import() {
    let requested = BTreeSet::from([Capability::FsRead, Capability::FsWrite]);
    let granted = BTreeSet::new();
    assert_eq!(
        missing_import_capabilities(
            ["wasi:filesystem/types@0.2.12"].into_iter(),
            &requested,
            &granted
        ),
        vec![Capability::FsRead]
    );
}

#[test]
fn requests_only_manifest_alternatives() {
    let requested = BTreeSet::from([Capability::FsWrite]);
    let granted = BTreeSet::new();
    assert_eq!(
        missing_import_capabilities(
            ["wasi:filesystem/types@0.2.12"].into_iter(),
            &requested,
            &granted
        ),
        vec![Capability::FsWrite]
    );
}

#[test]
fn ignores_satisfied_imports() {
    let requested = BTreeSet::from([Capability::FsRead, Capability::FsWrite]);
    let granted = BTreeSet::from([Capability::FsWrite]);
    assert!(missing_import_capabilities(
        ["wasi:filesystem/types@0.2.12"].into_iter(),
        &requested,
        &granted
    )
    .is_empty());
}

#[test]
fn rejects_partially_requested_alternatives() {
    let requested = BTreeSet::from([Capability::EditorRead, Capability::FsRead]);
    assert_eq!(
        unauthorized_imports(["wasi:filesystem/types@0.2.12"].into_iter(), &requested),
        vec!["wasi:filesystem/types@0.2.12"]
    );
}

#[test]
fn rejects_imports_beyond_manifest_requests() {
    let requested = BTreeSet::from([
        Capability::EditorRead,
        Capability::FsRead,
        Capability::FsWrite,
    ]);
    let imports = [
        "wasi:cli/environment@0.2.12",
        "wasi:filesystem/types@0.2.12",
        "kawi:plugin/editor-read@0.8.0",
        "kawi:plugin/editor-write@0.8.0",
        "kawi:plugin/editor-execute@0.8.0",
        "wasi:sockets/tcp@0.2.12",
    ];
    assert_eq!(
        unauthorized_imports(imports.into_iter(), &requested),
        vec![
            "kawi:plugin/editor-write@0.8.0",
            "kawi:plugin/editor-execute@0.8.0",
            "wasi:sockets/tcp@0.2.12"
        ]
    );
}

#[test]
fn allows_config_read_without_a_capability_request() {
    assert!(unauthorized_imports(
        ["kawi:plugin/config-read@0.8.0"].into_iter(),
        &BTreeSet::new()
    )
    .is_empty());
}

#[test]
fn rejects_unsupported_kawi_interface_versions() {
    let requested = BTreeSet::from(Capability::ALL);
    for version in ["0.1.0", "0.2.0", "0.3.0", "0.5.0"] {
        let import = format!("kawi:plugin/editor-read@{version}");
        assert_eq!(
            unauthorized_imports([import.as_str()].into_iter(), &requested),
            vec![import.as_str()]
        );
    }
}

#[test]
fn rejects_unknown_kawi_and_wasi_interfaces() {
    let requested = BTreeSet::from(Capability::ALL);
    assert_eq!(
        unauthorized_imports(
            ["kawi:plugin/unknown@0.8.0", "wasi:unknown/interface@0.2.12",].into_iter(),
            &requested
        ),
        vec!["kawi:plugin/unknown@0.8.0", "wasi:unknown/interface@0.2.12"]
    );
}

#[test]
fn wit_package_matches_the_import_table() {
    let wit = include_str!("../../../../kawi-plugin-sdk/wit/plugin.wit");
    assert!(wit.contains("package kawi:plugin@0.8.0;"));
    for rule in WASM_IMPORTS
        .iter()
        .filter(|rule| rule.prefix.starts_with("kawi:plugin/"))
    {
        let name = rule
            .prefix
            .trim_start_matches("kawi:plugin/")
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
        .filter(|rule| rule.prefix.starts_with("kawi:plugin/"))
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
                "kawi:plugin/editor-read@0.8.0",
                "kawi:plugin/editor-execute@0.8.0",
                "wasi:filesystem/types@0.2.12",
            ]
            .into_iter()
        ),
        BTreeSet::from([
            Capability::EditorRead,
            Capability::EditorExecute,
            Capability::FsRead,
            Capability::FsWrite
        ])
    );
}
