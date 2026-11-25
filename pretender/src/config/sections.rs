//! Remaining `[pretender.toml]` sections: bands, scope, execute, plugins,
//! output, patterns, and roles.
//!
//! # Responsibilities
//! Define the section structs, their kebab-case serde shape, and their
//! documented defaults (including the role matcher table).
//!
//! # Rationale
//! Split out of `config.rs` (wave 4 ratchet) so each file stays under the
//! file-size gate; struct bodies moved verbatim, with `Roles::default`
//! factored through [`role_matcher`] to keep the constructor within the
//! function-length threshold.

use genesis::config::ConfigValidation;
use serde::{Deserialize, Serialize};

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
    pub(crate) fn collect_validations(&self, out: &mut Vec<ConfigValidation>) {
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Execute {
    pub enabled: bool,
    pub coverage_cmd: Option<String>,
    pub mutation_cmd: Option<String>,
    pub test_cmd: Option<String>,
    pub test_report_path: Option<String>,
    pub test_timeout_s: u32,
    pub test_time_unit: TimeUnit,
}

impl Default for Execute {
    fn default() -> Self {
        Self {
            enabled: false,
            coverage_cmd: None,
            mutation_cmd: None,
            test_cmd: None,
            test_report_path: None,
            test_timeout_s: 600,
            test_time_unit: TimeUnit::Seconds,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TimeUnit {
    #[default]
    Seconds,
    Milliseconds,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Roles {
    pub test: RoleMatcher,
    pub library: RoleMatcher,
    pub script: RoleMatcher,
    pub generated: RoleMatcher,
    pub vendor: RoleMatcher,
    #[serde(rename = "unit-test")]
    pub unit_test: RoleMatcher,
    #[serde(rename = "integration-test")]
    pub integration_test: RoleMatcher,
}

/// Build a [`RoleMatcher`] from path globs and a class-name root; the
/// shared shape behind every entry of [`Roles::default`].
fn role_matcher(paths: &[&str], classname_root: &str) -> RoleMatcher {
    RoleMatcher {
        paths: paths.iter().map(|path| path.to_string()).collect(),
        classname_root: classname_root.to_string(),
    }
}

impl Default for Roles {
    fn default() -> Self {
        Self {
            test: role_matcher(&["tests/**", "**/*_test.*", "spec/**"], "tests"),
            library: role_matcher(&["pkg/**", "lib/**"], ""),
            script: role_matcher(&["scripts/**", "examples/**"], ""),
            generated: role_matcher(&["**/*.pb.go", "**/*_generated.*"], ""),
            vendor: role_matcher(&["vendor/**", "node_modules/**"], ""),
            unit_test: role_matcher(&["tests/unit/**"], ""),
            integration_test: role_matcher(&["tests/integration/**"], ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct RoleMatcher {
    pub paths: Vec<String>,
    pub classname_root: String,
}
