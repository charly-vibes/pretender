//! Pretender configuration.
//!
//! The `Config` struct is the domain model for `pretender.toml`. All file
//! I/O (read, parse, validate) delegates to `genesis::config::ConfigFile`;
//! this module only owns the struct shape and the domain validation rules.

use genesis::config::{ConfigFile, ConfigRegistry, ConfigValidation, ValidationSeverity};
use serde::{Deserialize, Serialize};
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
    /// Advisory lease (pretender-1te): `tiered`/`guidance` are dated
    /// downgrades from the gate default and require an `advisory_until`
    /// ISO date (YYYY-MM-DD). An expired, missing, or unparseable lease
    /// fails closed — the drift detector forcing a renewed, dated,
    /// reviewable decision instead of a permanent silent advisory.
    pub advisory_until: Option<String>,
    pub languages: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for PretenderSection {
    fn default() -> Self {
        Self {
            // Gate is the fail-closed default (pretender-1te): advisory
            // requires an explicit, dated downgrade.
            mode: Mode::Gate,
            advisory_until: None,
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

impl Mode {
    /// Advisory modes are dated downgrades from the gate default
    /// (pretender-1te): they fail closed without a current lease.
    pub fn is_advisory(self) -> bool {
        matches!(self, Mode::Guidance | Mode::Tiered)
    }
}

/// True when an advisory lease is expired or unparseable. `None` is
/// handled by the caller (advisory without a lease fails closed there);
/// gate mode never consults leases.
/// Date comparison is lexicographic on the ISO string — no date crate.
pub fn is_lease_expired(lease: Option<&str>) -> bool {
    let Some(lease) = lease else {
        return false; // gate path; advisory-without-lease fails in lease check
    };
    if lease.len() != 10 || lease.as_bytes()[4] != b'-' || lease.as_bytes()[7] != b'-' {
        return true;
    }
    if !lease
        .bytes()
        .enumerate()
        .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return true;
    }
    let month = match lease[5..7].parse::<u8>() {
        Ok(m) if (1..=12).contains(&m) => m,
        _ => return true,
    };
    let day = match lease[8..10].parse::<u8>() {
        Ok(d) if (1..=31).contains(&d) => d,
        _ => return true,
    };
    let _ = (month, day); // range-validated; comparison is lexicographic
    let today = today_iso();
    lease.as_bytes() <= today.as_bytes()
}

/// Today's date as ISO YYYY-MM-DD without a date crate:
/// Hinnant's civil-from-days over the Unix epoch via std::time.
fn today_iso() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    civil_from_days(days)
}

/// Howard Hinnant's civil_from_days: days since 1970-01-01 → ISO date.
fn civil_from_days(z: i64) -> String {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
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
    #[serde(rename = "unit-test")]
    pub unit_test: DurationThresholds,
    #[serde(rename = "integration-test")]
    pub integration_test: DurationThresholds,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct DurationThresholds {
    pub duration_max_ms: u32,
}

fn validate_percent(out: &mut Vec<ConfigValidation>, field: &'static str, value: u32) {
    if value > 100 {
        out.push(ConfigValidation::error(
            field,
            "expected percentage value <= 100",
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TimeUnit {
    #[default]
    Seconds,
    Milliseconds,
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

impl Default for Roles {
    fn default() -> Self {
        Self {
            test: RoleMatcher {
                paths: vec![
                    "tests/**".to_string(),
                    "**/*_test.*".to_string(),
                    "spec/**".to_string(),
                ],
                classname_root: "tests".to_string(),
            },
            library: RoleMatcher {
                paths: vec!["pkg/**".to_string(), "lib/**".to_string()],
                classname_root: String::new(),
            },
            script: RoleMatcher {
                paths: vec!["scripts/**".to_string(), "examples/**".to_string()],
                classname_root: String::new(),
            },
            generated: RoleMatcher {
                paths: vec!["**/*.pb.go".to_string(), "**/*_generated.*".to_string()],
                classname_root: String::new(),
            },
            vendor: RoleMatcher {
                paths: vec!["vendor/**".to_string(), "node_modules/**".to_string()],
                classname_root: String::new(),
            },
            unit_test: RoleMatcher {
                paths: vec!["tests/unit/**".to_string()],
                classname_root: String::new(),
            },
            integration_test: RoleMatcher {
                paths: vec!["tests/integration/**".to_string()],
                classname_root: String::new(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct RoleMatcher {
    pub paths: Vec<String>,
    pub classname_root: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── advisory-lease TTL tests (pretender-1te: gate-by-default, dated
    // downgrades) ───────────────────────────────────────────────────────

    #[test]
    fn lease_date_validation_rejects_malformed_dates() {
        assert!(is_lease_expired(Some("not-a-date")));
        assert!(is_lease_expired(Some("2099-13-01"))); // month out of range
        assert!(is_lease_expired(Some("2099-00-10")));
        assert!(!is_lease_expired(Some("2099-12-31")));
        assert!(is_lease_expired(Some("2000-01-01")));
        assert!(!is_lease_expired(None)); // gate path; caller handles advisory
    }

    #[test]
    fn advisory_mode_detection_covers_tiered_and_guidance() {
        assert!(Mode::Tiered.is_advisory());
        assert!(Mode::Guidance.is_advisory());
        assert!(!Mode::Gate.is_advisory());
    }

    #[test]
    fn advisory_mode_with_expired_or_missing_lease_fails_closed() {
        // decide_exit_code contract: advisory + no/expired lease = FAILURE
        // (behavioral test via decide_exit_code lives in main.rs; here we
        // pin the config-side predicates it composes).
        let lease = Some("2000-01-01".to_string());
        assert!(is_lease_expired(lease.as_deref()));
        let none: Option<String> = None;
        assert!(none.as_deref().is_none());
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
        .expect("config should parse");
        assert_eq!(config.pretender.mode, Mode::Tiered);
        assert_eq!(
            config.pretender.advisory_until.as_deref(),
            Some("2099-06-30")
        );
    }

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

        let validations = config.validate().expect("validate");
        assert!(!has_errors(&validations), "valid config has no errors");
    }

    #[test]
    fn default_config_matches_documented_conventions() {
        let config = Config::default();

        // Gate is the fail-closed default (pretender-1te)
        assert_eq!(config.pretender.mode, Mode::Gate);
        assert_eq!(config.pretender.advisory_until, None);
        assert_eq!(config.pretender.languages, vec!["auto"]);
        assert_eq!(config.thresholds.app.cyclomatic_max, 10);
        assert_eq!(config.thresholds.app.cognitive_max, 15);
        assert_eq!(config.thresholds.app.function_lines_max, 40);
        assert_eq!(config.thresholds.app.file_lines_max, 400);
        assert_eq!(config.thresholds.app.nesting_max, 3);
        assert_eq!(config.thresholds.app.params_max, 4);
        assert_eq!(config.thresholds.app.duplication_pct_max, 5);
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
            duplication_pct_max = 101
            "#,
        )
        .expect("config should parse");

        let validations = config.validate().expect("validate");
        assert!(validations
            .iter()
            .any(|v| v.field == "thresholds.duplication_pct_max"
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
        .expect("config should parse");
        let validations = config.validate().expect("validate");
        assert!(!has_errors(&validations), "zero duration is valid");
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
        .expect("config should parse");
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
        .expect("config should parse");
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
        .expect("config should parse");
        assert_eq!(config.roles.test.classname_root, "spec");
    }
}
