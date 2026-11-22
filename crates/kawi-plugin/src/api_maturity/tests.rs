use super::*;

#[test]
fn registry_names_stay_sorted_for_binary_search() {
    assert!(HOST_METHOD_NAMES.is_sorted());
    assert!(EVENT_NAMES.is_sorted());
    assert!(HOST_METHOD_NAMES.windows(2).all(|pair| pair[0] != pair[1]));
    assert!(EVENT_NAMES.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn every_registered_api_is_valid() {
    for (name, maturity) in HOST_METHOD_ENTRIES.iter().chain(EVENT_ENTRIES) {
        assert_eq!(maturity.validate(), Ok(()), "{name}");
    }
}

#[test]
fn introduction_minors_never_exceed_the_protocol_minor() {
    for (name, maturity) in HOST_METHOD_ENTRIES.iter().chain(EVENT_ENTRIES) {
        assert!(maturity.since <= crate::protocol::PROTOCOL_MINOR, "{name}");
    }
}

#[test]
fn lookups_find_the_first_and_last_entries() {
    let first_method = HOST_METHOD_NAMES.first().copied().unwrap();
    let last_method = HOST_METHOD_NAMES.last().copied().unwrap();
    assert_eq!(
        host_method_maturity(first_method),
        Some(HOST_METHOD_ENTRIES[0].1)
    );
    assert_eq!(
        host_method_maturity(last_method),
        Some(HOST_METHOD_ENTRIES[HOST_METHOD_ENTRIES.len() - 1].1)
    );

    let first_event = EVENT_NAMES.first().copied().unwrap();
    let last_event = EVENT_NAMES.last().copied().unwrap();
    assert_eq!(event_maturity(first_event), Some(EVENT_ENTRIES[0].1));
    assert_eq!(
        event_maturity(last_event),
        Some(EVENT_ENTRIES[EVENT_ENTRIES.len() - 1].1)
    );
}

#[test]
fn unknown_names_do_not_inherit_maturity() {
    assert_eq!(host_method_maturity("method.that_does_not_exist"), None);
    assert_eq!(event_maturity("event_that_does_not_exist"), None);
    assert_eq!(host_method_since("method.that_does_not_exist"), None);
    assert_eq!(event_since("event_that_does_not_exist"), None);
}

#[test]
fn stability_predicates_agree_with_the_stored_value() {
    let stable = ApiMaturity::stable(2);
    assert!(stable.is_stable());
    assert!(!stable.is_experimental());
    assert!(!stable.is_deprecated());

    let experimental = ApiMaturity::experimental(3);
    assert!(!experimental.is_stable());
    assert!(experimental.is_experimental());
    assert!(!experimental.is_deprecated());

    let deprecated = ApiMaturity::deprecated(3, 4, "new.method");
    assert!(!deprecated.is_stable());
    assert!(!deprecated.is_experimental());
    assert!(deprecated.is_deprecated());
}

#[test]
fn deprecated_metadata_needs_a_version_and_a_replacement() {
    assert_eq!(ApiMaturity::stable(3).validate(), Ok(()));
    assert_eq!(
        ApiMaturity::deprecated(3, 4, "new.method").validate(),
        Ok(())
    );
    assert_eq!(
        ApiMaturity::deprecated(3, 3, "new.method").validate(),
        Ok(())
    );

    let no_version = ApiMaturity {
        since: 3,
        stability: ApiStability::Deprecated,
        deprecated_since: None,
        replacement: Some("new.method"),
    };
    assert_eq!(
        no_version.validate(),
        Err(MaturityError::DeprecatedWithoutVersion)
    );

    let no_replacement = ApiMaturity {
        since: 3,
        stability: ApiStability::Deprecated,
        deprecated_since: Some(4),
        replacement: None,
    };
    assert_eq!(
        no_replacement.validate(),
        Err(MaturityError::DeprecatedWithoutReplacement)
    );

    let too_early = ApiMaturity::deprecated(4, 3, "new.method");
    assert_eq!(
        too_early.validate(),
        Err(MaturityError::DeprecatedBeforeIntroduction)
    );
}

#[test]
fn active_api_rejects_deprecation_fields() {
    let with_version = ApiMaturity {
        since: 3,
        stability: ApiStability::Stable,
        deprecated_since: Some(4),
        replacement: None,
    };
    assert_eq!(
        with_version.validate(),
        Err(MaturityError::ActiveWithDeprecationData)
    );

    let with_replacement = ApiMaturity {
        since: 3,
        stability: ApiStability::Experimental,
        deprecated_since: None,
        replacement: Some("new.method"),
    };
    assert_eq!(
        with_replacement.validate(),
        Err(MaturityError::ActiveWithDeprecationData)
    );
}

#[test]
fn lookups_are_exact_and_case_sensitive() {
    assert_eq!(host_method_maturity(""), None);
    assert_eq!(host_method_maturity("Editor.get_state"), None);
    assert_eq!(host_method_maturity("editor.get_state "), None);
    assert_eq!(host_method_maturity("editor.get"), None);
    assert_eq!(event_maturity("buffer_change"), None);
    assert_eq!(event_maturity("buffer_changedx"), None);
}

#[test]
fn since_accessors_cover_the_whole_protocol_universe() {
    for &(name, maturity) in HOST_METHOD_ENTRIES {
        assert_eq!(host_method_since(name), Some(maturity.since), "{name}");
    }
    for &(name, maturity) in EVENT_ENTRIES {
        assert_eq!(event_since(name), Some(maturity.since), "{name}");
    }
}

#[test]
fn stability_is_declared_not_derived_from_introduction_minor() {
    let stable_now = ApiMaturity::stable(crate::protocol::PROTOCOL_MINOR);
    let experimental_earlier = ApiMaturity::experimental(BASE_WASM_MINOR);

    assert_eq!(stable_now.validate(), Ok(()));
    assert_eq!(experimental_earlier.validate(), Ok(()));
    assert!(stable_now.is_stable());
    assert!(experimental_earlier.is_experimental());
}
