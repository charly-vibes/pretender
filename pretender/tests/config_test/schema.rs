//! Full-schema parsing: every `[pretender.toml]` section, unknown-key
//! tolerance, the advisory lease field, and the data-model thresholds.
//!
//! # Rationale
//! Moved from `src/config.rs` (wave 4); `parses_full_config_schema_and_
//! ignores_unknown_keys` was split into two functions over the same TOML
//! constant to stay within the test-role function-length threshold.

use genesis::config::ConfigFile as _;
use pretender::config::{Config, Mode, OutputFormat, TimeUnit};

const FULL_SCHEMA_TOML: &str = r#"
unknown_top_level = "ignored"

[pretender]
mode = "gate"
languages = ["python", "rust"]
exclude = ["vendor/**"]
future_key = true

[thresholds]
cyclomatic_max = 9
cognitive_max = 14
function_lines_max = 39
file_lines_max = 399
nesting_max = 2
params_max = 3
duplication_pct_max = 4

[thresholds.test]
cyclomatic_max = 3
function_lines_max = 80
nesting_max = 2
params_max = 2
cognitive_max = 5
duplication_pct_max = 30
min_assertions = 1

[thresholds.library]
exported_params_max = 3
exported_cyclomatic_max = 8
exported_lines_max = 30
require_docstring = true

[thresholds.script]
function_lines_max = 100
file_lines_max = 300

[bands]
cyclomatic = { green = 10, yellow = 15, red = 20 }
cognitive = { green = 15, yellow = 25, red = 40 }

[scope]
diff_only = true
diff_base = "origin/main"

[execute]
enabled = true
coverage_cmd = "pytest --cov --cov-report=xml"
mutation_cmd = "mutmut run"
test_cmd = "cargo test"
test_report_path = "target/test-report.xml"
test_timeout_s = 300
test_time_unit = "seconds"

[plugins]
languages = ["python", "javascript"]
metrics = ["ruff", "eslint"]

[output]
formats = ["human", "sarif"]
sarif_path = "pretender.sarif"

[roles]
test = { paths = ["tests/**"], classname_root = "tests" }
library = { paths = ["lib/**"] }
script = { paths = ["scripts/**"] }
generated = { paths = ["**/*_generated.*"] }
vendor = { paths = ["vendor/**"] }
unit-test = { paths = ["tests/unit/**"] }
integration-test = { paths = ["tests/integration/**"] }
"#;

fn full_schema() -> Config {
    Config::parse_str(FULL_SCHEMA_TOML).unwrap_or_else(|e| panic!("config should parse: {e:?}"))
}

#[test]
fn parses_full_config_schema_and_ignores_unknown_keys() {
    let config = full_schema();

    assert_eq!(config.pretender.mode, Mode::Gate);
    assert_eq!(config.pretender.languages, vec!["python", "rust"]);
    assert_eq!(config.thresholds.app.cyclomatic_max, 9);
    assert_eq!(config.thresholds.test.checks.min_assertions, Some(1));
    assert!(config.thresholds.library.require_docstring);
    assert_eq!(
        config
            .bands
            .cyclomatic
            .unwrap_or_else(|| panic!("bands.cyclomatic should be present"))
            .red,
        20
    );
    assert!(config.scope.diff_only);
}

#[test]
fn parses_full_schema_execute_output_roles_and_validates() {
    let config = full_schema();

    assert!(config.execute.enabled);
    assert_eq!(config.execute.test_cmd, Some("cargo test".to_string()));
    assert_eq!(
        config.execute.test_report_path,
        Some("target/test-report.xml".to_string())
    );
    assert_eq!(config.execute.test_timeout_s, 300);
    assert_eq!(config.execute.test_time_unit, TimeUnit::Seconds);
    assert_eq!(config.plugins.metrics, vec!["ruff", "eslint"]);
    assert_eq!(
        config.output.formats,
        vec![OutputFormat::Human, OutputFormat::Sarif]
    );
    assert_eq!(config.roles.test.paths, vec!["tests/**"]);
    assert_eq!(config.roles.test.classname_root, "tests");
    assert_eq!(config.roles.unit_test.paths, vec!["tests/unit/**"]);
    assert_eq!(
        config.roles.integration_test.paths,
        vec!["tests/integration/**"]
    );

    let validations = config
        .validate()
        .unwrap_or_else(|e| panic!("validate: {e:?}"));
    assert!(
        !pretender::config::has_errors(&validations),
        "valid config has no errors"
    );
}

#[test]
fn advisory_until_field_parses_from_toml() {
    let config = Config::parse_str(
        r#"
        [pretender]
        mode = "tiered"
        advisory_until = "2099-06-30"
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));
    assert_eq!(config.pretender.mode, Mode::Tiered);
    assert_eq!(
        config.pretender.advisory_until.as_deref(),
        Some("2099-06-30")
    );
}

// ── data-model thresholds (openspec add-data-model-checks §5) ─────────

#[test]
fn config_data_model_threshold_defaults_match_spec() {
    let config = Config::default();
    assert_eq!(config.thresholds.app.checks.excessive_fields_max, 10);
    assert!(!config.thresholds.app.checks.primitive_field_check);
    assert!(config.thresholds.app.checks.domain_patterns.is_empty());
}

#[test]
fn config_test_role_table_overrides_excessive_fields_max() {
    // "Test role disables excessive-field check" at merge level: the
    // role-table value is what EffectiveThresholds::for_role layers
    // over the app-level value.
    let config = Config::parse_str(
        r#"
        [thresholds]
        excessive_fields_max = 12

        [thresholds.test]
        excessive_fields_max = 0
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));
    assert_eq!(config.thresholds.app.checks.excessive_fields_max, 12);
    assert_eq!(config.thresholds.test.checks.excessive_fields_max, 0);
}

#[test]
fn config_domain_patterns_reaches_app_config_and_is_not_role_overridable() {
    // domain_patterns is a single app-level list: role tables neither
    // override nor merge into it (unknown-key tolerance drops the
    // role-table key).
    let config = Config::parse_str(
        r#"
        [thresholds]
        domain_patterns = ["^Id$", "Code$"]

        [thresholds.test]
        domain_patterns = ["TestOnly"]
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));
    assert_eq!(
        config.thresholds.app.checks.domain_patterns,
        vec!["^Id$".to_string(), "Code$".to_string()]
    );
}
