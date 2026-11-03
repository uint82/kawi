use super::*;

#[test]
fn builds_a_runtime_for_every_bundled_language() {
    for language in LanguageId::ALL {
        assert!(Runtime::new(language).is_some(), "{language:?}");
    }
}
