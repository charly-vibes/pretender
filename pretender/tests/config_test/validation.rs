//! Config validation rules: inverted bands and impossible percentages
//! must be flagged with error severity.
//!
//! # Rationale
//! Moved verbatim from `src/config.rs` (wave 4); expect/unwrap converted
//! to sanctioned panic-on-failure forms.

use genesis::config::{ConfigFile as _, ValidationSeverity};
use pretender::config::Config;

#[test]
fn validation_flags_inverted_bands() {
    let config = Config::parse_str(
        r#"
        [bands]
        cyclomatic = { green = 20, yellow = 10, red = 15 }
        "#,
    )
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));

    let validations = config
        .validate()
        .unwrap_or_else(|e| panic!("validate: {e:?}"));
    let band_issue = validations
        .iter()
        .find(|v| v.field == "bands.cyclomatic")
        .unwrap_or_else(|| panic!("bands.cyclomatic should be flagged"));
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
    .unwrap_or_else(|e| panic!("config should parse: {e:?}"));

    let validations = config
        .validate()
        .unwrap_or_else(|e| panic!("validate: {e:?}"));
    assert!(validations
        .iter()
        .any(|v| v.field == "thresholds.duplication_pct_max"
            && v.severity == ValidationSeverity::Error));
}
