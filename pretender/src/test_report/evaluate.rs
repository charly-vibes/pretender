//! Duration evaluation: per-testcase role resolution and threshold checks.

use super::{resolve_testcase_file, TestDurationFinding, TestTiming};
use crate::config::Config;
use crate::role_resolution::RoleDetector;
use crate::roles::{EffectiveThresholds, Role};
use std::path::{Path, PathBuf};

/// Evaluate test timings against configured duration thresholds.
///
/// Each testcase is resolved to a role via the `RoleDetector`. If the
/// resolved role has a non-zero `duration_max_ms` and the test's elapsed
/// time strictly exceeds it, a [`TestDurationFinding`] is emitted.
///
/// Tests that resolve to the base `test` role (or any role without a
/// duration threshold) are skipped.
pub fn evaluate_duration(
    timings: &[TestTiming],
    detector: &RoleDetector,
    config: &Config,
    search_root: &Path,
) -> Vec<TestDurationFinding> {
    timings
        .iter()
        .filter_map(|timing| evaluate_timing(timing, detector, config, search_root))
        .collect()
}

/// Evaluate one timing: resolve file + role, emit a finding when the
/// observed duration strictly exceeds the role's non-zero threshold.
fn evaluate_timing(
    timing: &TestTiming,
    detector: &RoleDetector,
    config: &Config,
    search_root: &Path,
) -> Option<TestDurationFinding> {
    let (file, role) = resolve_role(timing, detector, config, search_root);
    let effective = EffectiveThresholds::for_role(role, &config.thresholds);
    if effective.duration_max_ms > 0 && timing.duration_ms > effective.duration_max_ms {
        return Some(finding(timing, file, role, effective.duration_max_ms));
    }
    None
}

/// Resolve a testcase to (file, role): the file path first, base `test`
/// role when unresolved (verbatim original loop body).
fn resolve_role(
    timing: &TestTiming,
    detector: &RoleDetector,
    config: &Config,
    search_root: &Path,
) -> (Option<PathBuf>, Role) {
    let file = resolve_testcase_file(
        timing.file.as_deref(),
        &timing.classname,
        &config.roles.test.classname_root,
        search_root,
    );
    let role = match &file {
        Some(path) => detector.detect(path, ""),
        None => Role::Test,
    };
    (file, role)
}

/// Build the finding report for an exceeded threshold.
fn finding(
    timing: &TestTiming,
    file: Option<PathBuf>,
    role: Role,
    threshold_ms: u32,
) -> TestDurationFinding {
    TestDurationFinding {
        test_name: timing.name.clone(),
        file,
        role: role_name(role),
        observed_ms: timing.duration_ms,
        threshold_ms,
    }
}

/// The report-facing role name (table form of the original match; same
/// mapping for every [`Role`]).
fn role_name(role: Role) -> String {
    const NAMES: [(Role, &str); 8] = [
        (Role::App, "app"),
        (Role::Library, "library"),
        (Role::Test, "test"),
        (Role::Script, "script"),
        (Role::Generated, "generated"),
        (Role::Vendor, "vendor"),
        (Role::UnitTest, "unit-test"),
        (Role::IntegrationTest, "integration-test"),
    ];
    NAMES
        .iter()
        .find(|(candidate, _)| *candidate == role)
        .map(|(_, name)| (*name).to_string())
        .unwrap_or_default()
}
