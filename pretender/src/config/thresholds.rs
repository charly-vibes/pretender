//! The `[thresholds]` config tree: app/test/library/script/coupling tables,
//! the duration tables, their defaults, and their domain validation rules.
//!
//! # Responsibilities
//! Define the threshold structs, their serde shape (flat keys under
//! `[thresholds]` for app-level settings, sub-tables for role tables),
//! the pinned default values, and per-table validation.
//!
//! # Rationale
//! Split out of `config.rs` (wave 4 ratchet) so each file stays under the
//! file-size gate. The two oversized tables were decomposed into flattened
//! `checks` groups: Rust field paths gained one segment, the TOML spelling
//! is unchanged, and default values are identical.

use genesis::config::ConfigValidation;
use serde::Deserialize;

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
    pub(crate) fn collect_validations(&self, out: &mut Vec<ConfigValidation>) {
        validate_percent(
            out,
            "thresholds.duplication_pct_max",
            self.app.checks.duplication_pct_max,
        );
        validate_percent(
            out,
            "thresholds.test.duplication_pct_max",
            self.test.checks.duplication_pct_max,
        );
        self.coupling
            .collect_validations("thresholds.coupling", out);
        if self.app.checks.mut_ratio_max > 0.0
            && (self.app.checks.mut_ratio_max < 0.0 || self.app.checks.mut_ratio_max > 1.0)
        {
            out.push(ConfigValidation::error(
                "thresholds.mut_ratio_max",
                "expected value between 0.0 and 1.0",
            ));
        }
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
    pub void_mutators_max: u32,
    #[serde(flatten)]
    pub checks: AppChecksThresholds,
}

impl Default for AppThresholds {
    fn default() -> Self {
        Self {
            cyclomatic_max: 10,
            cognitive_max: 15,
            function_lines_max: 30,
            file_lines_max: 400,
            nesting_max: 3,
            params_max: 4,
            abc_max: 30,
            void_mutators_max: 3,
            checks: AppChecksThresholds::default(),
        }
    }
}

/// Discrete strictness rules layered flat under `[thresholds]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct AppChecksThresholds {
    pub duplication_pct_max: u32,
    pub mut_ratio_max: f64,
    pub unwrap_max: u32,
    pub bool_cluster_max: u32,
    pub primitive_param_check: bool,
    pub excessive_fields_max: u32,
    pub primitive_field_check: bool,
    pub domain_patterns: Vec<String>,
    pub inheritance_depth_max: u32,
}

impl Default for AppChecksThresholds {
    fn default() -> Self {
        Self {
            duplication_pct_max: 5,
            mut_ratio_max: 0.3,
            unwrap_max: 5,
            bool_cluster_max: 3,
            primitive_param_check: false,
            excessive_fields_max: 10,
            primitive_field_check: false,
            domain_patterns: Vec::new(),
            inheritance_depth_max: 2,
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
    #[serde(flatten)]
    pub checks: TestChecksThresholds,
}

impl Default for TestThresholds {
    fn default() -> Self {
        Self {
            cyclomatic_max: 3,
            function_lines_max: 80,
            nesting_max: 2,
            params_max: 2,
            cognitive_max: 5,
            checks: TestChecksThresholds::default(),
        }
    }
}

/// Discrete strictness rules layered flat under `[thresholds.test]`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct TestChecksThresholds {
    pub duplication_pct_max: u32,
    pub min_assertions: Option<u32>,
    pub mock_count_max: u32,
    pub void_mutators_max: u32,
    pub unwrap_max: u32,
    pub lazy_cluster_min: u32,
    pub excessive_fields_max: u32,
}

impl Default for TestChecksThresholds {
    fn default() -> Self {
        Self {
            duplication_pct_max: 30,
            min_assertions: Some(1),
            mock_count_max: 5,
            void_mutators_max: 0,
            unwrap_max: 5,
            lazy_cluster_min: 3,
            excessive_fields_max: 10,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
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
