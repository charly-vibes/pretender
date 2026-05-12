#![allow(dead_code)]

use crate::config::{Config, Thresholds};
use globset::{Glob, GlobMatcher};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    App,
    Library,
    Test,
    Script,
    Generated,
    Vendor,
}

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
            _ => None,
        }
    }
}

fn heuristic_role(path: &Path) -> Option<Role> {
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
    pub min_assertions: Option<u32>,
    pub exported_params_max: Option<u32>,
    pub exported_cyclomatic_max: Option<u32>,
    pub exported_lines_max: Option<u32>,
    pub require_docstring: bool,
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
            min_assertions: None,
            exported_params_max: None,
            exported_cyclomatic_max: None,
            exported_lines_max: None,
            require_docstring: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::Path;

    #[test]
    fn explicit_pragma_overrides_path_globs() {
        let detector = RoleDetector::new(&Config::default()).expect("valid default role globs");

        let role = detector.detect(
            Path::new("tests/generated/example_generated.py"),
            "# pretender: role=library\ndef helper(): pass\n",
        );

        assert_eq!(role, Role::Library);
    }

    #[test]
    fn pragma_accepts_whitespace_and_block_comment_syntax() {
        let detector = RoleDetector::new(&Config::default()).expect("valid default role globs");

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
        let config = Config::load_from_str(
            r#"
            [roles]
            script = { paths = ["tests/manual/**"] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid configured role globs");

        let role = detector.detect(Path::new("tests/manual/demo.py"), "def demo(): pass\n");

        assert_eq!(role, Role::Script);
    }

    #[test]
    fn falls_back_to_heuristics_then_app() {
        let config = Config::load_from_str(
            r#"
            [roles]
            test = { paths = [] }
            library = { paths = [] }
            script = { paths = [] }
            generated = { paths = [] }
            vendor = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("empty globs are valid");

        assert_eq!(detector.detect(Path::new("spec/widget.py"), ""), Role::Test);
        assert_eq!(detector.detect(Path::new("src/widget.py"), ""), Role::App);
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
}
