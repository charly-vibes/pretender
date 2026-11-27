//! File roles and the per-role effective threshold merge.
//!
//! # Responsibilities
//! Define the [`Role`] enum, the path-heuristic fallback [`heuristic_role`],
//! and the merge of a role's config table over the app baseline
//! ([`EffectiveThresholds::for_role`]).
//!
//! # Rationale
//! Heuristic and overlay rules are data tables, so adding a pattern or a
//! role tweak is one row instead of another ladder arm.

use crate::config::{AppThresholds, Thresholds};
use std::path::Path;

/// The role a file plays; the `[thresholds]` role tables are keyed by it.
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

/// One heuristic rule: directory hints matched boundary-aware against the
/// normalized path, plus file-name suffix and substring hints.
struct RolePattern {
    role: Role,
    path_hints: &'static [&'static str],
    name_suffixes: &'static [&'static str],
    name_contains: &'static [&'static str],
}

/// Heuristic rows in evaluation order; the first hit wins, so vendor
/// outranks generated markers and unit/integration outrank the broad test
/// rule. Path hints keep their trailing slash so only directory segments
/// match — a bare file named `vendor` or `spec` is not a role hit.
const ROLE_PATTERNS: &[RolePattern] = &[
    RolePattern {
        role: Role::Vendor,
        path_hints: &["vendor/", "node_modules/"],
        name_suffixes: &[],
        name_contains: &[],
    },
    RolePattern {
        role: Role::Generated,
        path_hints: &[],
        name_suffixes: &[
            "_generated.py",
            "_generated.js",
            "_generated.ts",
            "_generated.rs",
            ".pb.go",
        ],
        name_contains: &[],
    },
    RolePattern {
        role: Role::UnitTest,
        path_hints: &["tests/unit/", "test/unit/"],
        name_suffixes: &[],
        name_contains: &[],
    },
    RolePattern {
        role: Role::IntegrationTest,
        path_hints: &["tests/integration/", "test/integration/"],
        name_suffixes: &[],
        name_contains: &[],
    },
    RolePattern {
        role: Role::Test,
        path_hints: &["tests/", "spec/"],
        name_suffixes: &[],
        name_contains: &["_test."],
    },
    RolePattern {
        role: Role::Library,
        path_hints: &["pkg/", "lib/"],
        name_suffixes: &[],
        name_contains: &[],
    },
    RolePattern {
        role: Role::Script,
        path_hints: &["scripts/", "examples/"],
        name_suffixes: &[],
        name_contains: &[],
    },
];

impl RolePattern {
    fn matches(&self, normalized: &str, file_name: &str) -> bool {
        self.path_hints
            .iter()
            .any(|hint| path_hint_matches(normalized, hint))
            || self
                .name_suffixes
                .iter()
                .any(|hint| file_name.ends_with(hint))
            || self
                .name_contains
                .iter()
                .any(|hint| file_name.contains(hint))
    }
}

/// Boundary-aware contains: `hint` must start the path or follow a `/`,
/// so `vendor/` hits `vendor/a.py` and `a/vendor/b.py`, never `xvendor/`.
fn path_hint_matches(normalized: &str, hint: &str) -> bool {
    normalized
        .match_indices(hint)
        .any(|(at, _)| at == 0 || normalized.as_bytes()[at - 1] == b'/')
}

/// True for shared test-support helper modules: any boundary-aligned
/// directory segment named `support` (pretender-jg9y). These files wire
/// the system under test; `mock_count`'s budget stays for leaf tests.
pub fn is_test_support_path(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
        .split('/')
        .any(|segment| segment == "support")
}

/// Best-effort role for paths the configured role tables didn't claim.
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
    ROLE_PATTERNS
        .iter()
        .find(|pattern| pattern.matches(&normalized, &file_name))
        .map(|pattern| pattern.role)
}

/// Function-size and complexity caps; role tables override these values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionLimits {
    pub cyclomatic_max: u32,
    pub cognitive_max: u32,
    pub function_lines_max: u32,
    pub nesting_max: u32,
    pub params_max: u32,
    pub abc_max: u32,
}

/// File-scope caps for the line, void-mutator, unwrap and field-count checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLimits {
    pub file_lines_max: u32,
    pub void_mutators_max: u32,
    pub unwrap_max: u32,
    pub excessive_fields_max: u32,
}

/// Exported-unit rules; only the library table sets them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRules {
    pub exported_params_max: Option<u32>,
    pub exported_cyclomatic_max: Option<u32>,
    pub exported_lines_max: Option<u32>,
    pub require_docstring: bool,
}

/// The thresholds one file is checked against: the app baseline with the
/// file's role table layered on top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveThresholds {
    pub function: FunctionLimits,
    pub file: FileLimits,
    pub min_assertions: Option<u32>,
    pub exports: ExportRules,
    pub duration_max_ms: u32,
}

/// Overlay a role applies on top of the app baseline; roles without an
/// entry keep the baseline untouched.
type RoleOverlay = fn(EffectiveThresholds, &Thresholds) -> EffectiveThresholds;

/// Indexed by `Role as usize` (declaration order of the enum).
const ROLE_OVERLAYS: [Option<RoleOverlay>; 8] = [
    None,                                              // App
    Some(EffectiveThresholds::apply_library),          // Library
    Some(EffectiveThresholds::apply_test),             // Test
    Some(EffectiveThresholds::apply_script),           // Script
    None,                                              // Generated
    None,                                              // Vendor
    Some(EffectiveThresholds::apply_unit_test),        // UnitTest
    Some(EffectiveThresholds::apply_integration_test), // IntegrationTest
];

impl EffectiveThresholds {
    /// Merge `role`'s config table over the app baseline.
    pub fn for_role(role: Role, thresholds: &Thresholds) -> Self {
        let effective = Self::from_app(thresholds);
        if let Some(overlay) = ROLE_OVERLAYS[role as usize] {
            return overlay(effective, thresholds);
        }
        effective
    }

    /// The app table is the baseline; export rules and durations start off.
    fn from_app(thresholds: &Thresholds) -> Self {
        Self {
            function: FunctionLimits::from_app(&thresholds.app),
            file: FileLimits::from_app(&thresholds.app),
            min_assertions: None,
            exports: ExportRules::none(),
            duration_max_ms: 0,
        }
    }

    /// The test table; unit/integration inherit it and overlay a duration.
    fn apply_test(mut self, thresholds: &Thresholds) -> Self {
        let test = &thresholds.test;
        self.function.cyclomatic_max = test.cyclomatic_max;
        self.function.cognitive_max = test.cognitive_max;
        self.function.function_lines_max = test.function_lines_max;
        self.function.nesting_max = test.nesting_max;
        self.function.params_max = test.params_max;
        self.min_assertions = test.checks.min_assertions;
        self.file.void_mutators_max = test.checks.void_mutators_max;
        self.file.unwrap_max = test.checks.unwrap_max;
        self.file.excessive_fields_max = test.checks.excessive_fields_max;
        self
    }

    fn apply_unit_test(self, thresholds: &Thresholds) -> Self {
        Self {
            duration_max_ms: thresholds.unit_test.duration_max_ms,
            ..self.apply_test(thresholds)
        }
    }

    fn apply_integration_test(self, thresholds: &Thresholds) -> Self {
        Self {
            duration_max_ms: thresholds.integration_test.duration_max_ms,
            ..self.apply_test(thresholds)
        }
    }

    fn apply_library(mut self, thresholds: &Thresholds) -> Self {
        self.exports = ExportRules {
            exported_params_max: Some(thresholds.library.exported_params_max),
            exported_cyclomatic_max: Some(thresholds.library.exported_cyclomatic_max),
            exported_lines_max: Some(thresholds.library.exported_lines_max),
            require_docstring: thresholds.library.require_docstring,
        };
        self
    }

    fn apply_script(mut self, thresholds: &Thresholds) -> Self {
        self.function.function_lines_max = thresholds.script.function_lines_max;
        self.file.file_lines_max = thresholds.script.file_lines_max;
        self
    }
}

impl FunctionLimits {
    fn from_app(app: &AppThresholds) -> Self {
        Self {
            cyclomatic_max: app.cyclomatic_max,
            cognitive_max: app.cognitive_max,
            function_lines_max: app.function_lines_max,
            nesting_max: app.nesting_max,
            params_max: app.params_max,
            abc_max: app.abc_max,
        }
    }
}

impl FileLimits {
    fn from_app(app: &AppThresholds) -> Self {
        Self {
            file_lines_max: app.file_lines_max,
            void_mutators_max: app.void_mutators_max,
            unwrap_max: app.checks.unwrap_max,
            excessive_fields_max: app.checks.excessive_fields_max,
        }
    }
}

impl ExportRules {
    fn none() -> Self {
        Self {
            exported_params_max: None,
            exported_cyclomatic_max: None,
            exported_lines_max: None,
            require_docstring: false,
        }
    }
}
