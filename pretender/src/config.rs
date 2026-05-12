#![allow(dead_code)]

use miette::Diagnostic;
use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    #[diagnostic(code(pretender::config::read_failed))]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse pretender.toml: {0}")]
    #[diagnostic(code(pretender::config::parse_failed))]
    Parse(#[from] toml::de::Error),

    #[error("invalid config at {path}: {message}")]
    #[diagnostic(code(pretender::config::validation_failed))]
    Validation { path: &'static str, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
}

impl Config {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::load_from_str(&source)
    }

    pub fn load_from_str(source: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(source)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.bands.validate()?;
        self.thresholds.validate()?;
        if self.output.formats.is_empty() {
            return Err(ConfigError::Validation {
                path: "output.formats",
                message: "expected at least one output format".to_string(),
            });
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pretender: PretenderSection::default(),
            thresholds: Thresholds::default(),
            bands: Bands::default(),
            scope: Scope::default(),
            execute: Execute::default(),
            plugins: Plugins::default(),
            output: Output::default(),
            roles: Roles::default(),
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    #[serde(flatten)]
    pub app: AppThresholds,
    pub test: TestThresholds,
    pub library: LibraryThresholds,
    pub script: ScriptThresholds,
}

impl Thresholds {
    fn validate(&self) -> Result<(), ConfigError> {
        validate_percent(
            "thresholds.duplication_pct_max",
            self.app.duplication_pct_max,
        )?;
        validate_percent("thresholds.coverage_line_min", self.app.coverage_line_min)?;
        validate_percent(
            "thresholds.coverage_branch_min",
            self.app.coverage_branch_min,
        )?;
        validate_percent("thresholds.mutation_min", self.app.mutation_min)?;
        validate_percent(
            "thresholds.test.duplication_pct_max",
            self.test.duplication_pct_max,
        )?;
        Ok(())
    }
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            app: AppThresholds::default(),
            test: TestThresholds::default(),
            library: LibraryThresholds::default(),
            script: ScriptThresholds::default(),
        }
    }
}

fn validate_percent(path: &'static str, value: u32) -> Result<(), ConfigError> {
    if value <= 100 {
        Ok(())
    } else {
        Err(ConfigError::Validation {
            path,
            message: "expected percentage value <= 100".to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct AppThresholds {
    pub cyclomatic_max: u32,
    pub cognitive_max: u32,
    pub function_lines_max: u32,
    pub file_lines_max: u32,
    pub nesting_max: u32,
    pub params_max: u32,
    pub duplication_pct_max: u32,
    pub mi_min: u32,
    pub coverage_line_min: u32,
    pub coverage_branch_min: u32,
    pub mutation_min: u32,
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
            duplication_pct_max: 5,
            mi_min: 20,
            coverage_line_min: 80,
            coverage_branch_min: 70,
            mutation_min: 60,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Band {
    pub green: u32,
    pub yellow: u32,
    pub red: u32,
}

impl Band {
    fn validate(&self, path: &'static str) -> Result<(), ConfigError> {
        if self.green <= self.yellow && self.yellow <= self.red {
            Ok(())
        } else {
            Err(ConfigError::Validation {
                path,
                message: "expected green <= yellow <= red".to_string(),
            })
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
    fn validate(&self) -> Result<(), ConfigError> {
        if let Some(band) = self.cyclomatic {
            band.validate("bands.cyclomatic")?;
        }
        if let Some(band) = self.cognitive {
            band.validate("bands.cognitive")?;
        }
        Ok(())
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
        let config = Config::load_from_str(
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
    fn validation_rejects_inverted_bands_with_miette_diagnostic() {
        let error = Config::load_from_str(
            r#"
            [bands]
            cyclomatic = { green = 20, yellow = 10, red = 15 }
            "#,
        )
        .expect_err("invalid bands should fail validation");

        let rendered = format!("{error}");
        assert!(rendered.contains("bands.cyclomatic"));
    }

    #[test]
    fn validation_rejects_impossible_percentages() {
        let error = Config::load_from_str(
            r#"
            [thresholds]
            coverage_line_min = 101
            "#,
        )
        .expect_err("percentage thresholds above 100 should fail validation");

        assert!(format!("{error}").contains("thresholds.coverage_line_min"));
    }
}
