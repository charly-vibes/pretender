//! Role resolution tests. Path-only assertions go through `path_role` so
//! each heuristic case stays a single line within the structural budgets.

use super::*;
use crate::config::Config;
use std::path::Path;

fn detector(config: &Config) -> RoleDetector {
    RoleDetector::new(config).unwrap_or_else(|e| panic!("valid role globs: {e}"))
}

fn config(toml: &str) -> Config {
    Config::parse_str(toml).unwrap_or_else(|e| panic!("config parses: {e}"))
}

/// Config with the test-role path tables emptied so only heuristics apply.
fn heuristics_only_config() -> Config {
    config(
        r#"
        [roles]
        test = { paths = [] }
        unit-test = { paths = [] }
        integration-test = { paths = [] }
        "#,
    )
}

/// Role resolved from the path alone (empty source, no pragmas).
fn path_role(detector: &RoleDetector, path: &str) -> Role {
    detector.detect(Path::new(path), "")
}

#[test]
fn explicit_pragma_overrides_path_globs() {
    let detector = detector(&Config::default());

    let role = detector.detect(
        Path::new("tests/generated/example_generated.py"),
        "# pretender: role=library\ndef helper(): pass\n",
    );

    assert_eq!(role, Role::Library);
}

#[test]
fn pragma_accepts_whitespace_and_block_comment_syntax() {
    let detector = detector(&Config::default());

    assert_eq!(
        detector.detect(
            Path::new("src/generated.pb.go"),
            "/* pretender: role = vendor */\n"
        ),
        Role::Vendor
    );
}

#[test]
fn configured_path_globs_override_heuristics() {
    let config = config(
        r#"
        [roles]
        script = { paths = ["tests/manual/**"] }
        "#,
    );
    let detector = detector(&config);

    let role = detector.detect(Path::new("tests/manual/demo.py"), "def demo(): pass\n");

    assert_eq!(role, Role::Script);
}

#[test]
fn falls_back_to_heuristics_then_app() {
    let config = config(
        r#"
        [roles]
        test = { paths = [] }
        library = { paths = [] }
        script = { paths = [] }
        generated = { paths = [] }
        vendor = { paths = [] }
        "#,
    );
    let detector = detector(&config);

    assert_eq!(path_role(&detector, "spec/widget.py"), Role::Test);
    assert_eq!(path_role(&detector, "src/widget.py"), Role::App);
}

#[test]
fn pragma_unit_test_underscore_form() {
    let detector = detector(&Config::default());
    let role = detector.detect(Path::new("src/random.py"), "// pretender: role=unit_test\n");
    assert_eq!(role, Role::UnitTest);
}

#[test]
fn pragma_unit_test_bare_form() {
    let detector = detector(&Config::default());
    let role = detector.detect(Path::new("src/random.py"), "// pretender: role=unit\n");
    assert_eq!(role, Role::UnitTest);
}

#[test]
fn pragma_integration_test_kebab_form() {
    let detector = detector(&Config::default());
    let role = detector.detect(
        Path::new("src/random.py"),
        "# pretender: role=integration-test\n",
    );
    assert_eq!(role, Role::IntegrationTest);
}

#[test]
fn pragma_integration_test_underscore_form() {
    let detector = detector(&Config::default());
    let role = detector.detect(
        Path::new("src/random.py"),
        "/* pretender: role = integration_test */\n",
    );
    assert_eq!(role, Role::IntegrationTest);
}

#[test]
fn glob_configured_unit_test_paths() {
    let config = config(
        r#"
        [roles.unit-test]
        paths = ["tests/unit/**"]
        [roles.integration-test]
        paths = ["tests/integration/**"]
        "#,
    );
    let detector = detector(&config);

    assert_eq!(
        path_role(&detector, "tests/unit/test_widget.py"),
        Role::UnitTest
    );
    assert_eq!(
        path_role(&detector, "tests/integration/test_api.py"),
        Role::IntegrationTest
    );
}

#[test]
fn heuristic_unit_test_paths() {
    let detector = detector(&heuristics_only_config());

    assert_eq!(
        path_role(&detector, "tests/unit/test_widget.py"),
        Role::UnitTest
    );
    assert_eq!(
        path_role(&detector, "test/unit/test_widget.py"),
        Role::UnitTest
    );
    assert_eq!(
        path_role(&detector, "src/tests/unit/test_widget.py"),
        Role::UnitTest
    );
}

#[test]
fn heuristic_integration_test_paths() {
    let detector = detector(&heuristics_only_config());

    assert_eq!(
        path_role(&detector, "tests/integration/test_api.py"),
        Role::IntegrationTest
    );
    assert_eq!(
        path_role(&detector, "test/integration/test_api.py"),
        Role::IntegrationTest
    );
    assert_eq!(
        path_role(&detector, "myapp/tests/integration/test_api.py"),
        Role::IntegrationTest
    );
}

#[test]
fn src_unit_helper_not_misclassified_as_unit_test() {
    // Design requirement: Do NOT match the broad **/unit/**
    let detector = detector(&heuristics_only_config());

    // src/unit/helper.rs is NOT a test file, should get App
    assert_eq!(path_role(&detector, "src/unit/helper.rs"), Role::App);
    // src/tests/unit.rs is a test file (not tests/unit/ subdir), should be Test
    assert_eq!(path_role(&detector, "src/tests/unit.rs"), Role::Test);
}

#[test]
fn unit_test_role_overrides_base_test_heuristic() {
    // tests/unit/ should be UnitTest, not base Test
    let detector = detector(&heuristics_only_config());

    assert_eq!(
        path_role(&detector, "tests/unit/test_widget.py"),
        Role::UnitTest
    );
    assert_eq!(path_role(&detector, "tests/test_widget.py"), Role::Test);
}
