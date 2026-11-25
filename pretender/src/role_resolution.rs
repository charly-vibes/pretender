//! Role resolution for a source file.
//!
//! Resolves a file's [`Role`] from explicit `pretender:`/`pretender-role:`
//! pragmas, configured path globs, and heuristic path rules, in that
//! precedence order.
//!
//! # Responsibilities
//! [`RoleDetector`] compiles the configured globs (with specificity
//! ordering) and applies pragma > glob > heuristic > App fallback;
//! the pragma and comment parsers recognize both supported spellings.
//!
//! # Rationale
//! Detection machinery (config, globset, source-text coupling) is split out
//! of `roles.rs` so that module keeps only the pure role and threshold
//! tables; bodies moved verbatim to keep role assignment identical.
use crate::config::Config;
use crate::roles::{heuristic_role, Role};
use globset::{Glob, GlobMatcher};
use std::path::Path;

pub struct RoleDetector {
    patterns: Vec<RolePattern>,
}

impl RoleDetector {
    pub fn new(config: &Config) -> Result<Self, globset::Error> {
        let mut patterns = Vec::new();
        add_patterns(&mut patterns, Role::Test, &config.roles.test.paths)?;
        add_patterns(&mut patterns, Role::Library, &config.roles.library.paths)?;
        add_patterns(&mut patterns, Role::Script, &config.roles.script.paths)?;
        add_patterns(
            &mut patterns,
            Role::Generated,
            &config.roles.generated.paths,
        )?;
        add_patterns(&mut patterns, Role::Vendor, &config.roles.vendor.paths)?;
        add_patterns(&mut patterns, Role::UnitTest, &config.roles.unit_test.paths)?;
        add_patterns(
            &mut patterns,
            Role::IntegrationTest,
            &config.roles.integration_test.paths,
        )?;
        Ok(Self { patterns })
    }

    pub fn detect(&self, path: &Path, source: &str) -> Role {
        explicit_pragma_role(source)
            .or_else(|| self.glob_role(path))
            .or_else(|| heuristic_role(path))
            .unwrap_or(Role::App)
    }

    fn glob_role(&self, path: &Path) -> Option<Role> {
        self.patterns
            .iter()
            .filter(|pattern| pattern.matcher.is_match(path))
            .max_by_key(|pattern| pattern.specificity)
            .map(|pattern| pattern.role)
    }
}

struct RolePattern {
    role: Role,
    matcher: GlobMatcher,
    specificity: usize,
}

fn add_patterns(
    patterns: &mut Vec<RolePattern>,
    role: Role,
    globs: &[String],
) -> Result<(), globset::Error> {
    for pattern in globs {
        patterns.push(RolePattern {
            role,
            matcher: Glob::new(pattern)?.compile_matcher(),
            specificity: pattern
                .chars()
                .filter(|c| !matches!(c, '*' | '?' | '[' | ']'))
                .count(),
        });
    }
    Ok(())
}

fn explicit_pragma_role(source: &str) -> Option<Role> {
    source
        .lines()
        .take(8)
        .filter_map(comment_text)
        .find_map(parse_role_pragma)
}

fn comment_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix('#')
        .or_else(|| trimmed.strip_prefix("//"))
        .or_else(|| trimmed.strip_prefix("/*"))
        .map(str::trim)
}

fn parse_role_pragma(comment: &str) -> Option<Role> {
    let lower = comment
        .trim()
        .trim_end_matches("*/")
        .trim()
        .to_ascii_lowercase();
    let value = if let Some(rest) = lower.strip_prefix("pretender:") {
        let (key, value) = rest.split_once('=')?;
        (key.trim() == "role").then_some(value.trim())?
    } else {
        lower.strip_prefix("pretender-role:").map(str::trim)?
    };
    Role::parse(value)
}

impl Role {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "app" => Some(Self::App),
            "library" | "lib" => Some(Self::Library),
            "test" | "tests" | "spec" => Some(Self::Test),
            "script" | "scripts" => Some(Self::Script),
            "generated" => Some(Self::Generated),
            "vendor" => Some(Self::Vendor),
            "unit-test" | "unit_test" | "unit" => Some(Self::UnitTest),
            "integration-test" | "integration_test" | "integration" => Some(Self::IntegrationTest),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::Path;

    fn detector(config: &Config) -> RoleDetector {
        RoleDetector::new(config).unwrap_or_else(|e| panic!("valid role globs: {e}"))
    }

    fn config(toml: &str) -> Config {
        Config::parse_str(toml).unwrap_or_else(|e| panic!("config parses: {e}"))
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

        assert_eq!(detector.detect(Path::new("spec/widget.py"), ""), Role::Test);
        assert_eq!(detector.detect(Path::new("src/widget.py"), ""), Role::App);
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
            detector.detect(Path::new("tests/unit/test_widget.py"), ""),
            Role::UnitTest
        );
        assert_eq!(
            detector.detect(Path::new("tests/integration/test_api.py"), ""),
            Role::IntegrationTest
        );
    }

    #[test]
    fn heuristic_unit_test_paths() {
        let config = config(
            r#"
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        );
        let detector = detector(&config);

        assert_eq!(
            detector.detect(Path::new("tests/unit/test_widget.py"), ""),
            Role::UnitTest
        );
        assert_eq!(
            detector.detect(Path::new("test/unit/test_widget.py"), ""),
            Role::UnitTest
        );
        assert_eq!(
            detector.detect(Path::new("src/tests/unit/test_widget.py"), ""),
            Role::UnitTest
        );
    }

    #[test]
    fn heuristic_integration_test_paths() {
        let config = config(
            r#"
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        );
        let detector = detector(&config);

        assert_eq!(
            detector.detect(Path::new("tests/integration/test_api.py"), ""),
            Role::IntegrationTest
        );
        assert_eq!(
            detector.detect(Path::new("test/integration/test_api.py"), ""),
            Role::IntegrationTest
        );
        assert_eq!(
            detector.detect(Path::new("myapp/tests/integration/test_api.py"), ""),
            Role::IntegrationTest
        );
    }

    #[test]
    fn src_unit_helper_not_misclassified_as_unit_test() {
        // Design requirement: Do NOT match the broad **/unit/**
        let config = config(
            r#"
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        );
        let detector = detector(&config);

        // src/unit/helper.rs is NOT a test file, should get App
        assert_eq!(
            detector.detect(Path::new("src/unit/helper.rs"), ""),
            Role::App
        );
        // src/tests/unit.rs is a test file (not tests/unit/ subdir), should be Test
        assert_eq!(
            detector.detect(Path::new("src/tests/unit.rs"), ""),
            Role::Test
        );
    }

    #[test]
    fn unit_test_role_overrides_base_test_heuristic() {
        // tests/unit/ should be UnitTest, not base Test
        let config = config(
            r#"
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        );
        let detector = detector(&config);

        assert_eq!(
            detector.detect(Path::new("tests/unit/test_widget.py"), ""),
            Role::UnitTest
        );
        assert_eq!(
            detector.detect(Path::new("tests/test_widget.py"), ""),
            Role::Test
        );
    }
}
