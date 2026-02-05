//! Test that ValidatorRegistry is Sync + Send

fn assert_sync_send<T: Sync + Send>() {}

#[test]
fn test_validator_registry_is_sync_send() {
    assert_sync_send::<agnix_core::ValidatorRegistry>();
}

#[test]
fn test_lint_config_is_sync_send() {
    assert_sync_send::<agnix_core::LintConfig>();
}
