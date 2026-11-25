//! Documented-default and ConfigFile-wiring pins: the default config must
//! match the shipped conventions, execute/roles defaults, registry and
//! path wiring, and duration-table validation.
//!
//! # Rationale
//! Moved verbatim from `src/config.rs` (wave 4); expect/unwrap converted
//! to sanctioned panic-on-failure forms.

use genesis::config::ConfigFile as _;
use pretender::config::{build_registry, Band, Config, TimeUnit};

#[test]
fn default_config_matches_documented_conventions() {
    let config = Config::default();

    // Gate is the fail-closed default (pretender-1te)
    assert_eq!(config.pretender.mode, pretender::config::Mode::Gate);
    assert_eq!(config.pretender.advisory_until, None);
    assert_eq!(config.pretender.languages, vec!["auto"]);
    assert_eq!(config.thresholds.app.cyclomatic_max, 10);
    assert_eq!(config.thresholds.app.cognitive_max, 15);
    // update-app-threshold-defaults: function length tightened 40 → 30
    assert_eq!(config.thresholds.app.function_lines_max, 30);
    assert_eq!(config.thresholds.app.file_lines_max, 400);
    assert_eq!(config.thresholds.app.nesting_max, 3);
    assert_eq!(config.thresholds.app.params_max, 4);
    // update-app-threshold-defaults: previously-disabled checks enabled
    // at the values documented in openspec/changes/CONVENTIONS.md
    assert_eq!(config.thresholds.app.void_mutators_max, 3);
    assert_eq!(config.thresholds.app.checks.mut_ratio_max, 0.3);
    assert_eq!(config.thresholds.app.checks.unwrap_max, 5);
    assert_eq!(config.thresholds.app.checks.bool_cluster_max, 3);
    assert_eq!(config.thresholds.app.checks.inheritance_depth_max, 2);
    // test table: unwrap has no app fallback in EffectiveThresholds::for_role,
    // so the test row is required (matches the shipped unwrap spec)
    assert_eq!(config.thresholds.test.checks.unwrap_max, 5);
    assert_eq!(config.thresholds.test.checks.mock_count_max, 5);
    assert_eq!(config.thresholds.test.checks.lazy_cluster_min, 3);
    let cyclomatic = config
        .bands
        .cyclomatic
        .unwrap_or_else(|| panic!("bands.cyclomatic should be present"));
    assert_eq!(
        cyclomatic,
        Band {
            green: 10,
            yellow: 15,
            red: 20
        }
    );
    let cognitive = config
        .bands
        .cognitive
        .unwrap_or_else(|| panic!("bands.cognitive should be present"));
    assert_eq!(
        cognitive,
        Band {
            green: 15,
            yellow: 25,
            red: 40
        }
    );
    assert_eq!(
        config.roles.vendor.paths,
        vec!["vendor/**", "node_modules/**"]
    );
    assert_eq!(config.roles.test.classname_root, "tests");
    assert_eq!(config.roles.library.classname_root, "");
    assert_eq!(config.roles.unit_test.classname_root, "");
    assert_eq!(config.roles.integration_test.classname_root, "");
    assert_eq!(config.execute.test_timeout_s, 600);
    assert_eq!(config.execute.test_time_unit, TimeUnit::Seconds);
    assert_eq!(config.execute.test_cmd, None);
    assert_eq!(config.execute.test_report_path, None);
    assert_eq!(config.thresholds.unit_test.duration_max_ms, 0);
    assert_eq!(config.thresholds.integration_test.duration_max_ms, 0);
}

#[test]
fn execute_time_unit_defaults_to_seconds() {
    let config = Config::default();
    assert_eq!(config.execute.test_time_unit, TimeUnit::Seconds);
}

#[test]
fn execute_parses_time_unit_kebab_case() {
    let config = Config::parse_str(
        r#"
        [execute]
        test_time_unit = "milliseconds"
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));
    assert_eq!(config.execute.test_time_unit, TimeUnit::Milliseconds);
}

#[test]
fn role_matcher_classname_root_defaults_to_empty() {
    let config = Config::parse_str(
        r#"
        [roles.unit-test]
        paths = ["tests/unit/**"]
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));
    assert_eq!(config.roles.unit_test.classname_root, "");
}

#[test]
fn role_matcher_classname_root_can_be_set() {
    let config = Config::parse_str(
        r#"
        [roles.test]
        paths = ["tests/**"]
        classname_root = "spec"
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));
    assert_eq!(config.roles.test.classname_root, "spec");
}

#[test]
fn configfile_path_is_repo_root_pretender_toml() {
    let root = std::path::Path::new("/tmp/repo");
    assert_eq!(Config::path(root), root.join("pretender.toml"));
}

#[test]
fn build_registry_registers_pretender() {
    let registry = build_registry();
    assert!(registry.is_registered("pretender"));
    assert_eq!(registry.marker("pretender"), Some("pretender.toml"));
}

#[test]
fn validation_passes_for_duration_thresholds_zero() {
    let config = Config::parse_str(
        r#"
        [thresholds.unit-test]
        duration_max_ms = 0
        [thresholds.integration-test]
        duration_max_ms = 0
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));
    let validations = config
        .validate()
        .unwrap_or_else(|e| panic!("validate: {e:?}"));
    assert!(
        !pretender::config::has_errors(&validations),
        "zero duration is valid"
    );
}
