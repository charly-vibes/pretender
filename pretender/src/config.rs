//! Pretender configuration.
//!
//! The `Config` struct is the domain model for `pretender.toml`. All file
//! I/O (read, parse, validate) delegates to `genesis::config::ConfigFile`;
//! this module owns the top-level struct shape, the mode/advisory-lease
//! section, and domain validation wiring. The thresholds tree lives in
//! [`thresholds`], the remaining sections in [`sections`].

mod sections;
mod thresholds;

pub use sections::{
    Band, Bands, Execute, MockPatterns, Output, OutputFormat, Patterns, Plugins, RoleMatcher,
    Roles, Scope, TimeUnit,
};
pub use thresholds::{
    AppChecksThresholds, AppThresholds, CouplingThresholds, DurationThresholds, LibraryThresholds,
    ScriptThresholds, TestChecksThresholds, TestThresholds, Thresholds,
};

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

// ── Mode section ──────────────────────────────────────────────────────

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
    /// Honor .gitignore (plus global/core.excludesFile and .git/info/exclude)
    /// when walking directories, so generated/vendored/scratch dirs stop
    /// being scanned without per-repo exclude lists (pretender-094).
    /// Explicitly passed file paths and --staged are never filtered.
    pub respect_gitignore: bool,
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
            respect_gitignore: true,
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
    !lease_is_well_formed(lease) || lease.as_bytes() <= today_iso().as_bytes()
}

/// True when `lease` is a structurally valid ISO YYYY-MM-DD date: fixed
/// length and dash positions, digits elsewhere, month and day in range.
/// Values are range-checked; comparison stays lexicographic.
fn lease_is_well_formed(lease: &str) -> bool {
    lease_shape_ok(lease) && lease_digits_ok(lease) && lease_month_ok(lease) && lease_day_ok(lease)
}

/// Fixed length and dash positions of an ISO date.
fn lease_shape_ok(lease: &str) -> bool {
    let bytes = lease.as_bytes();
    bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-'
}

/// Every position except the dashes holds an ASCII digit.
fn lease_digits_ok(lease: &str) -> bool {
    lease
        .bytes()
        .enumerate()
        .all(|(i, byte)| i == 4 || i == 7 || byte.is_ascii_digit())
}

/// Month component parses into 1..=12.
fn lease_month_ok(lease: &str) -> bool {
    matches!(lease[5..7].parse::<u8>(), Ok(month) if (1..=12).contains(&month))
}

/// Day component parses into 1..=31.
fn lease_day_ok(lease: &str) -> bool {
    matches!(lease[8..10].parse::<u8>(), Ok(day) if (1..=31).contains(&day))
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
