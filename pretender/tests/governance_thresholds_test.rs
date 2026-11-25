//! Governance gate .19.3: pretender's effective thresholds must stay within
//! the reviewed bounds in governance/threshold-floor.toml.
//!
//! # Responsibilities
//! Load the governance gate and assert the empty-violation contract; the
//! floor/ceiling framework lives in the `floor_gate` child module.
//!
//! # Rationale
//! The framework was split into `governance_thresholds_test/floor_gate.rs`
//! (moved verbatim) so both files stay within the gate's per-file size
//! limit; the test's semantics are unchanged.

// util's repo-root helper is used only by this entry point.
#[allow(dead_code)]
#[path = "governance/util.rs"]
mod util;

#[path = "governance_thresholds_test/floor_gate.rs"]
mod floor_gate;

/// .19.3: every effective threshold must sit inside its reviewed ceiling and
/// floor; the empty violation list proves the merge stayed governed.
#[test]
fn effective_thresholds_respect_governance_bounds() {
    let Some(root) = util::repo_root() else {
        return;
    };
    let gate = floor_gate::Gate::load(&root);
    let violations = gate.violations();
    assert!(
        violations.is_empty(),
        "governance threshold violations:\n{}",
        violations.join("\n")
    );
}
