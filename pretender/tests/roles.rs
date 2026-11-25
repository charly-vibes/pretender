//! Role-threshold merge tests.
//!
//! # Responsibilities
//! Pin the `[thresholds]` role-table merge in
//! [`EffectiveThresholds::for_role`]: overrides land before metric
//! evaluation, test sub-roles inherit the test table and overlay only the
//! duration, and the library/script tables hit their own fields.
//!
//! # Rationale
//! Moved verbatim out of `src/roles.rs` (wave 4 ratchet): the items are
//! lib-pub, so the assertions run unchanged as an integration target.

use pretender::config::Config;
use pretender::roles::{EffectiveThresholds, Role};

fn config(toml: &str) -> Config {
    Config::parse_str(toml).unwrap_or_else(|e| panic!("config parses: {e}"))
}

#[test]
fn role_thresholds_apply_overrides_before_metric_evaluation() {
    let config = Config::default();

    let app = EffectiveThresholds::for_role(Role::App, &config.thresholds);
    let test = EffectiveThresholds::for_role(Role::Test, &config.thresholds);
    let library = EffectiveThresholds::for_role(Role::Library, &config.thresholds);
    let script = EffectiveThresholds::for_role(Role::Script, &config.thresholds);

    assert_eq!(app.function.cyclomatic_max, 10);
    assert_eq!(test.function.cyclomatic_max, 3);
    assert_eq!(test.min_assertions, Some(1));
    assert_eq!(library.exports.exported_cyclomatic_max, Some(8));
    assert!(library.exports.require_docstring);
    assert_eq!(script.function.function_lines_max, 100);
}

#[test]
fn unit_test_effective_thresholds_inherit_test_and_overlay_duration() {
    let config = config(
        r#"
        [thresholds.unit-test]
        duration_max_ms = 100
        [thresholds.integration-test]
        duration_max_ms = 500
        "#,
    );

    let unit = EffectiveThresholds::for_role(Role::UnitTest, &config.thresholds);
    let integration = EffectiveThresholds::for_role(Role::IntegrationTest, &config.thresholds);
    let base_test = EffectiveThresholds::for_role(Role::Test, &config.thresholds);

    // Sub-roles inherit test thresholds
    assert_eq!(
        unit.function.cyclomatic_max,
        base_test.function.cyclomatic_max
    );
    assert_eq!(unit.min_assertions, base_test.min_assertions);

    // Sub-roles overlay duration_max_ms
    assert_eq!(unit.duration_max_ms, 100);
    assert_eq!(integration.duration_max_ms, 500);

    // Base test has no duration threshold
    assert_eq!(base_test.duration_max_ms, 0);
}
