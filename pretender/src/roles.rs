use crate::config::Thresholds;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    App,
    Library,
    Test,
    Script,
    Generated,
    Vendor,
    UnitTest,
    IntegrationTest,
}

pub(crate) fn heuristic_role(path: &Path) -> Option<Role> {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if normalized.contains("/vendor/")
        || normalized.starts_with("vendor/")
        || normalized.contains("/node_modules/")
        || normalized.starts_with("node_modules/")
    {
        Some(Role::Vendor)
    } else if file_name.ends_with("_generated.py")
        || file_name.ends_with("_generated.js")
        || file_name.ends_with("_generated.ts")
        || file_name.ends_with("_generated.rs")
        || file_name.ends_with(".pb.go")
    {
        Some(Role::Generated)
    } else if normalized.starts_with("tests/unit/")
        || normalized.contains("/tests/unit/")
        || normalized.starts_with("test/unit/")
        || normalized.contains("/test/unit/")
    {
        Some(Role::UnitTest)
    } else if normalized.starts_with("tests/integration/")
        || normalized.contains("/tests/integration/")
        || normalized.starts_with("test/integration/")
        || normalized.contains("/test/integration/")
    {
        Some(Role::IntegrationTest)
    } else if normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.starts_with("spec/")
        || normalized.contains("/spec/")
        || file_name.contains("_test.")
    {
        Some(Role::Test)
    } else if normalized.starts_with("pkg/")
        || normalized.contains("/pkg/")
        || normalized.starts_with("lib/")
        || normalized.contains("/lib/")
    {
        Some(Role::Library)
    } else if normalized.starts_with("scripts/")
        || normalized.contains("/scripts/")
        || normalized.starts_with("examples/")
        || normalized.contains("/examples/")
    {
        Some(Role::Script)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveThresholds {
    pub cyclomatic_max: u32,
    pub cognitive_max: u32,
    pub function_lines_max: u32,
    pub file_lines_max: u32,
    pub nesting_max: u32,
    pub params_max: u32,
    pub duplication_pct_max: u32,
    pub abc_max: u32,
    pub min_assertions: Option<u32>,
    pub exported_params_max: Option<u32>,
    pub exported_cyclomatic_max: Option<u32>,
    pub exported_lines_max: Option<u32>,
    pub require_docstring: bool,
    pub void_mutators_max: u32,
    pub unwrap_max: u32,
    pub excessive_fields_max: u32,
    pub duration_max_ms: u32,
}

impl EffectiveThresholds {
    pub fn for_role(role: Role, thresholds: &Thresholds) -> Self {
        let mut effective = Self::from_app(thresholds);
        match role {
            Role::App | Role::Generated | Role::Vendor => {}
            Role::Test => {
                effective.cyclomatic_max = thresholds.test.cyclomatic_max;
                effective.cognitive_max = thresholds.test.cognitive_max;
                effective.function_lines_max = thresholds.test.function_lines_max;
                effective.nesting_max = thresholds.test.nesting_max;
                effective.params_max = thresholds.test.params_max;
                effective.duplication_pct_max = thresholds.test.duplication_pct_max;
                effective.min_assertions = thresholds.test.min_assertions;
                effective.void_mutators_max = thresholds.test.void_mutators_max;
                effective.unwrap_max = thresholds.test.unwrap_max;
                effective.excessive_fields_max = thresholds.test.excessive_fields_max;
            }
            Role::UnitTest => {
                effective.cyclomatic_max = thresholds.test.cyclomatic_max;
                effective.cognitive_max = thresholds.test.cognitive_max;
                effective.function_lines_max = thresholds.test.function_lines_max;
                effective.nesting_max = thresholds.test.nesting_max;
                effective.params_max = thresholds.test.params_max;
                effective.duplication_pct_max = thresholds.test.duplication_pct_max;
                effective.min_assertions = thresholds.test.min_assertions;
                effective.void_mutators_max = thresholds.test.void_mutators_max;
                effective.unwrap_max = thresholds.test.unwrap_max;
                effective.excessive_fields_max = thresholds.test.excessive_fields_max;
                effective.duration_max_ms = thresholds.unit_test.duration_max_ms;
            }
            Role::IntegrationTest => {
                effective.cyclomatic_max = thresholds.test.cyclomatic_max;
                effective.cognitive_max = thresholds.test.cognitive_max;
                effective.function_lines_max = thresholds.test.function_lines_max;
                effective.nesting_max = thresholds.test.nesting_max;
                effective.params_max = thresholds.test.params_max;
                effective.duplication_pct_max = thresholds.test.duplication_pct_max;
                effective.min_assertions = thresholds.test.min_assertions;
                effective.void_mutators_max = thresholds.test.void_mutators_max;
                effective.unwrap_max = thresholds.test.unwrap_max;
                effective.excessive_fields_max = thresholds.test.excessive_fields_max;
                effective.duration_max_ms = thresholds.integration_test.duration_max_ms;
            }
            Role::Library => {
                effective.exported_params_max = Some(thresholds.library.exported_params_max);
                effective.exported_cyclomatic_max =
                    Some(thresholds.library.exported_cyclomatic_max);
                effective.exported_lines_max = Some(thresholds.library.exported_lines_max);
                effective.require_docstring = thresholds.library.require_docstring;
            }
            Role::Script => {
                effective.function_lines_max = thresholds.script.function_lines_max;
                effective.file_lines_max = thresholds.script.file_lines_max;
            }
        }
        effective
    }

    fn from_app(thresholds: &Thresholds) -> Self {
        Self {
            cyclomatic_max: thresholds.app.cyclomatic_max,
            cognitive_max: thresholds.app.cognitive_max,
            function_lines_max: thresholds.app.function_lines_max,
            file_lines_max: thresholds.app.file_lines_max,
            nesting_max: thresholds.app.nesting_max,
            params_max: thresholds.app.params_max,
            duplication_pct_max: thresholds.app.duplication_pct_max,
            abc_max: thresholds.app.abc_max,
            min_assertions: None,
            exported_params_max: None,
            exported_cyclomatic_max: None,
            exported_lines_max: None,
            require_docstring: false,
            void_mutators_max: thresholds.app.void_mutators_max,
            unwrap_max: thresholds.app.unwrap_max,
            excessive_fields_max: thresholds.app.excessive_fields_max,
            duration_max_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

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

        assert_eq!(app.cyclomatic_max, 10);
        assert_eq!(test.cyclomatic_max, 3);
        assert_eq!(test.min_assertions, Some(1));
        assert_eq!(library.exported_cyclomatic_max, Some(8));
        assert!(library.require_docstring);
        assert_eq!(script.function_lines_max, 100);
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
        assert_eq!(unit.cyclomatic_max, base_test.cyclomatic_max);
        assert_eq!(unit.min_assertions, base_test.min_assertions);

        // Sub-roles overlay duration_max_ms
        assert_eq!(unit.duration_max_ms, 100);
        assert_eq!(integration.duration_max_ms, 500);

        // Base test has no duration threshold
        assert_eq!(base_test.duration_max_ms, 0);
    }
}
