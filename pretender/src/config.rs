//! Pretender configuration.
//!
//! The `Config` struct is the domain model for `pretender.toml`. All file
//! I/O (read, parse, validate) delegates to `genesis::config::ConfigFile`;
//! this module only owns the struct shape and the domain validation rules.

use genesis::config::{ConfigFile, ConfigRegistry, ConfigValidation, ValidationSeverity};
use serde::Deserialize;
use std::path::{Path, PathBuf};

// ── Config struct ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub pretender: PretenderSection,
    pub thresholds: Thresholds,
    pub bands: Bands,
    pub scope: Scope,
    pub execute: Execute,
    pub plugins: Plugins,
    pub output: Output,
    pub roles: Roles,
    pub patterns: Patterns,
}

impl Config {
    /// Parse a config from a TOML source string without touching the
    /// filesystem. Test convenience; runtime loading goes through
    /// [`genesis::config::ConfigFile::read_from`] via the `ConfigStore`.
    #[cfg(test)]
    pub fn parse_str(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }
}

impl ConfigFile for Config {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join("pretender.toml")
    }

    fn validate(&self) -> Result<Vec<ConfigValidation>, genesis::config::ConfigError> {
        let mut results = Vec::new();
        self.bands.collect_validations(&mut results);
        self.thresholds.collect_validations(&mut results);
        if self.output.formats.is_empty() {
            results.push(ConfigValidation::error(
                "output.formats",
                "expected at least one output format",
            ));
        }
        Ok(results)
    }
}

/// Build a [`ConfigRegistry`] with pretender's config registered.
///
/// Tools register their config struct at startup so the shared
/// `ConfigStore` can discover and validate it alongside other suite tools.
pub fn build_registry() -> ConfigRegistry {
    let mut registry = ConfigRegistry::new();
    registry.register::<Config>("pretender", "pretender.toml");
    registry
}

/// Return `true` if `validations` contains any error-severity entry.
pub fn has_errors(validations: &[ConfigValidation]) -> bool {
    validations
        .iter()
        .any(|v| v.severity == ValidationSeverity::Error)
}

// ── Sections ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct PretenderSection {
    pub mode: Mode,
    pub languages: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for PretenderSection {
    fn default() -> Self {
        Self {
            mode: Mode::Tiered,
            languages: vec!["auto".to_string()],
            exclude: vec![
                "vendor/**".to_string(),
                "node_modules/**".to_string(),
                "**/*_generated.*".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Guidance,
    Tiered,
    Gate,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    #[serde(flatten)]
    pub app: AppThresholds,
    pub test: TestThresholds,
    pub library: LibraryThresholds,
    pub script: ScriptThresholds,
    pub coupling: CouplingThresholds,
}

impl Thresholds {
    fn collect_validations(&self, out: &mut Vec<ConfigValidation>) {
        validate_percent(
            out,
            "thresholds.duplication_pct_max",
            self.app.duplication_pct_max,
        );
        validate_percent(
            out,
            "thresholds.coverage_line_min",
            self.app.coverage_line_min,
        );
        validate_percent(
            out,
            "thresholds.coverage_branch_min",
            self.app.coverage_branch_min,
        );
        validate_percent(out, "thresholds.mutation_min", self.app.mutation_min);
        validate_percent(
            out,
            "thresholds.test.duplication_pct_max",
            self.test.duplication_pct_max,
        );
        self.coupling
            .collect_validations("thresholds.coupling", out);
        if self.app.mut_ratio_max > 0.0
            && (self.app.mut_ratio_max < 0.0 || self.app.mut_ratio_max > 1.0)
        {
            out.push(ConfigValidation::error(
                "thresholds.mut_ratio_max",
                "expected value between 0.0 and 1.0",
            ));
        }
    }
}

fn validate_percent(out: &mut Vec<ConfigValidation>, field: &'static str, value: u32) {
    if value > 100 {
        out.push(ConfigValidation::error(
            field,
            "expected percentage value <= 100",
        ));
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct AppThresholds {
    pub cyclomatic_max: u32,
    pub cognitive_max: u32,
    pub function_lines_max: u32,
    pub file_lines_max: u32,
    pub nesting_max: u32,
    pub params_max: u32,
    pub abc_max: u32,
    pub duplication_pct_max: u32,
    pub mi_min: u32,
    pub coverage_line_min: u32,
    pub coverage_branch_min: u32,
    pub mutation_min: u32,
    pub void_mutators_max: u32,
    pub mut_ratio_max: f64,
    pub unwrap_max: u32,
    pub bool_cluster_max: u32,
    pub primitive_param_check: bool,
    pub inheritance_depth_max: u32,
}

impl Default for AppThresholds {
    fn default() -> Self {
        Self {
            cyclomatic_max: 10,
            cognitive_max: 15,
            function_lines_max: 40,
            file_lines_max: 400,
            nesting_max: 3,
            params_max: 4,
            abc_max: 30,
            duplication_pct_max: 5,
            mi_min: 20,
            coverage_line_min: 80,
            coverage_branch_min: 70,
            mutation_min: 60,
            void_mutators_max: 0,
            mut_ratio_max: 0.0,
            unwrap_max: 0,
            bool_cluster_max: 0,
            primitive_param_check: false,
            inheritance_depth_max: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct TestThresholds {
    pub cyclomatic_max: u32,
    pub function_lines_max: u32,
    pub nesting_max: u32,
    pub params_max: u32,
    pub cognitive_max: u32,
    pub duplication_pct_max: u32,
    pub min_assertions: Option<u32>,
    pub mock_count_max: u32,
    pub void_mutators_max: u32,
    pub unwrap_max: u32,
    pub lazy_cluster_min: u32,
}

impl Default for TestThresholds {
    fn default() -> Self {
        Self {
            cyclomatic_max: 3,
            function_lines_max: 80,
            nesting_max: 2,
            params_max: 2,
            cognitive_max: 5,
            duplication_pct_max: 30,
            min_assertions: Some(1),
            mock_count_max: 0,
            void_mutators_max: 0,
            unwrap_max: 0,
            lazy_cluster_min: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct LibraryThresholds {
    pub exported_params_max: u32,
    pub exported_cyclomatic_max: u32,
    pub exported_lines_max: u32,
    pub require_docstring: bool,
}

impl Default for LibraryThresholds {
    fn default() -> Self {
        Self {
            exported_params_max: 3,
            exported_cyclomatic_max: 8,
            exported_lines_max: 30,
            require_docstring: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ScriptThresholds {
    pub function_lines_max: u32,
    pub file_lines_max: u32,
}

impl Default for ScriptThresholds {
    fn default() -> Self {
        Self {
            function_lines_max: 100,
            file_lines_max: 300,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct CouplingThresholds {
    pub ce_max: u32,
    pub ca_max: u32,
    pub cbo_max: u32,
    pub lcom_hs_max: u32,
    pub cycle_detection: bool,
}

impl CouplingThresholds {
    fn collect_validations(&self, field: &'static str, out: &mut Vec<ConfigValidation>) {
        if self.lcom_hs_max > 100 {
            out.push(ConfigValidation::error(
                format!("{field}.lcom_hs_max"),
                "expected percentage value <= 100",
            ));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Band {
    pub green: u32,
    pub yellow: u32,
    pub red: u32,
}

impl Band {
    fn collect_validations(&self, field: &'static str, out: &mut Vec<ConfigValidation>) {
        if !(self.green <= self.yellow && self.yellow <= self.red) {
            out.push(ConfigValidation::error(
                field,
                "expected green <= yellow <= red",
            ));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Bands {
    pub cyclomatic: Option<Band>,
    pub cognitive: Option<Band>,
}

impl Bands {
    fn collect_validations(&self, out: &mut Vec<ConfigValidation>) {
        if let Some(band) = self.cyclomatic {
            band.collect_validations("bands.cyclomatic", out);
        }
        if let Some(band) = self.cognitive {
            band.collect_validations("bands.cognitive", out);
        }
    }
}

impl Default for Bands {
    fn default() -> Self {
        Self {
            cyclomatic: Some(Band {
                green: 10,
                yellow: 15,
                red: 20,
            }),
            cognitive: Some(Band {
                green: 15,
                yellow: 25,
                red: 40,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Scope {
    pub diff_only: bool,
    pub diff_base: String,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            diff_only: true,
            diff_base: "origin/main".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct Execute {
    pub enabled: bool,
    pub coverage_cmd: Option<String>,
    pub mutation_cmd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Plugins {
    pub languages: Vec<String>,
    pub metrics: Vec<String>,
}

impl Default for Plugins {
    fn default() -> Self {
        Self {
            languages: vec![
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "go".to_string(),
                "rust".to_string(),
            ],
            metrics: vec![
                "eslint".to_string(),
                "ruff".to_string(),
                "clippy".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Output {
    pub formats: Vec<OutputFormat>,
    pub sarif_path: String,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            formats: vec![OutputFormat::Human, OutputFormat::Sarif],
            sarif_path: "pretender.sarif".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct Patterns {
    pub mock: MockPatterns,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct MockPatterns {
    pub extra: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Human,
    Json,
    Sarif,
    Junit,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Roles {
    pub test: RoleMatcher,
    pub library: RoleMatcher,
    pub script: RoleMatcher,
    pub generated: RoleMatcher,
    pub vendor: RoleMatcher,
}

impl Default for Roles {
    fn default() -> Self {
        Self {
            test: RoleMatcher {
                paths: vec![
                    "tests/**".to_string(),
                    "**/*_test.*".to_string(),
                    "spec/**".to_string(),
                ],
            },
            library: RoleMatcher {
                paths: vec!["pkg/**".to_string(), "lib/**".to_string()],
            },
            script: RoleMatcher {
                paths: vec!["scripts/**".to_string(), "examples/**".to_string()],
            },
            generated: RoleMatcher {
                paths: vec!["**/*.pb.go".to_string(), "**/*_generated.*".to_string()],
            },
            vendor: RoleMatcher {
                paths: vec!["vendor/**".to_string(), "node_modules/**".to_string()],
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct RoleMatcher {
    pub paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config_schema_and_ignores_unknown_keys() {
        let config = Config::parse_str(
            r#"
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
            mi_min = 21
            coverage_line_min = 81
            coverage_branch_min = 71
            mutation_min = 61

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

            [plugins]
            languages = ["python", "javascript"]
            metrics = ["ruff", "eslint"]

            [output]
            formats = ["human", "sarif"]
            sarif_path = "pretender.sarif"

            [roles]
            test = { paths = ["tests/**"] }
            library = { paths = ["lib/**"] }
            script = { paths = ["scripts/**"] }
            generated = { paths = ["**/*_generated.*"] }
            vendor = { paths = ["vendor/**"] }
            "#,
        )
        .expect("config should parse");

        assert_eq!(config.pretender.mode, Mode::Gate);
        assert_eq!(config.pretender.languages, vec!["python", "rust"]);
        assert_eq!(config.thresholds.app.cyclomatic_max, 9);
        assert_eq!(config.thresholds.test.min_assertions, Some(1));
        assert!(config.thresholds.library.require_docstring);
        assert_eq!(config.bands.cyclomatic.unwrap().red, 20);
        assert!(config.scope.diff_only);
        assert!(config.execute.enabled);
        assert_eq!(config.plugins.metrics, vec!["ruff", "eslint"]);
        assert_eq!(
            config.output.formats,
            vec![OutputFormat::Human, OutputFormat::Sarif]
        );
        assert_eq!(config.roles.test.paths, vec!["tests/**"]);

        let validations = config.validate().expect("validate");
        assert!(!has_errors(&validations), "valid config has no errors");
    }

    #[test]
    fn default_config_matches_documented_conventions() {
        let config = Config::default();

        assert_eq!(config.pretender.mode, Mode::Tiered);
        assert_eq!(config.pretender.languages, vec!["auto"]);
        assert_eq!(config.thresholds.app.cyclomatic_max, 10);
        assert_eq!(config.thresholds.app.cognitive_max, 15);
        assert_eq!(config.thresholds.app.function_lines_max, 40);
        assert_eq!(config.thresholds.app.file_lines_max, 400);
        assert_eq!(config.thresholds.app.nesting_max, 3);
        assert_eq!(config.thresholds.app.params_max, 4);
        assert_eq!(config.thresholds.app.duplication_pct_max, 5);
        assert_eq!(config.thresholds.app.mi_min, 20);
        assert_eq!(config.thresholds.app.coverage_line_min, 80);
        assert_eq!(config.thresholds.app.coverage_branch_min, 70);
        assert_eq!(config.thresholds.app.mutation_min, 60);
        assert_eq!(
            config.bands.cyclomatic.unwrap(),
            Band {
                green: 10,
                yellow: 15,
                red: 20
            }
        );
        assert_eq!(
            config.bands.cognitive.unwrap(),
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
    }

    #[test]
    fn validation_flags_inverted_bands() {
        let config = Config::parse_str(
            r#"
            [bands]
            cyclomatic = { green = 20, yellow = 10, red = 15 }
            "#,
        )
        .expect("config should parse");

        let validations = config.validate().expect("validate");
        let band_issue = validations
            .iter()
            .find(|v| v.field == "bands.cyclomatic")
            .expect("bands.cyclomatic should be flagged");
        assert_eq!(band_issue.severity, ValidationSeverity::Error);
        assert!(band_issue.message.contains("green <= yellow <= red"));
    }

    #[test]
    fn validation_flags_impossible_percentages() {
        let config = Config::parse_str(
            r#"
            [thresholds]
            coverage_line_min = 101
            "#,
        )
        .expect("config should parse");

        let validations = config.validate().expect("validate");
        assert!(validations
            .iter()
            .any(|v| v.field == "thresholds.coverage_line_min"
                && v.severity == ValidationSeverity::Error));
    }

    #[test]
    fn configfile_path_is_repo_root_pretender_toml() {
        let root = Path::new("/tmp/repo");
        assert_eq!(Config::path(root), root.join("pretender.toml"));
    }

    #[test]
    fn build_registry_registers_pretender() {
        let registry = build_registry();
        assert!(registry.is_registered("pretender"));
        assert_eq!(registry.marker("pretender"), Some("pretender.toml"));
    }
}
