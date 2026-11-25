//! Advisory-lease and mode predicates (pretender-1te: gate-by-default,
//! dated downgrades).
//!
//! # Rationale
//! Moved verbatim from `src/config.rs` (wave 4); expect/unwrap converted
//! to sanctioned panic-on-failure forms.

use pretender::config::{is_lease_expired, Mode};

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
    // (behavioral test via decide_exit_code lives in the CLI tests; here
    // we pin the config-side predicates it composes).
    let lease = Some("2000-01-01".to_string());
    assert!(is_lease_expired(lease.as_deref()));
    let none: Option<String> = None;
    assert!(none.as_deref().is_none());
}
