//! `check` min-assertions test-role exemption tests.
//!
//! # Responsibilities
//! Pin the test-role threshold interplay: min assertions reported/enforced
//! for `#[test]` functions, fixture/helper/unattributed exemptions, and
//! test-role acceptance when an assertion is present.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;

#[test]
fn test_check_reports_min_assertions_for_test_role() {
    let (_dir, staged) = write_temp_file(
        "tests/test_no_assertions.py",
        "def test_missing_assertion():\n    helper()\n",
    );

    let output = check(&staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test_missing_assertion"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("min_assertions"), "stdout: {stdout}");
}

#[test]
fn test_check_exempts_fixture_unit_from_min_assertions() {
    let (dir, staged) = write_temp_file(
        "tests/test_fixture_sample.py",
        "def simple():\n    return 1\n\ndef with_branch(x):\n    if x:\n        return 1\n    return 0\n",
    );

    let output = check_in(&dir, &staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("min_assertions"),
        "fixture units must be exempt from min_assertions; stdout: {stdout}"
    );
}

#[test]
fn test_check_exempts_helper_unit_from_min_assertions() {
    let (_dir, staged) = write_temp_file(
        "tests/test_helpers_sample.py",
        "def tempdir():\n    return 1\n",
    );

    let output = check(&staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "helper exemption must not fail the run; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("min_assertions"),
        "helper units must be exempt from min_assertions; stdout: {stdout}"
    );
}

#[test]
fn test_check_rust_test_attribute_keeps_min_assertions() {
    // Guard: #[test]-attributed fns keep the rule even with non-test names.
    let (dir, staged) = write_temp_file(
        "tests/sample_test.rs",
        "#[test]\nfn adds() {\n    let a = 2;\n}\n",
    );

    let output = check_in(&dir, &staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("min_assertions"),
        "#[test] fn must stay subject to min_assertions; stdout: {stdout}"
    );
}

#[test]
fn test_check_exempts_unattributed_rust_unit_from_min_assertions() {
    let (_dir, staged) = write_temp_file(
        "tests/sample_probe_test.rs",
        "fn probe_helper() {\n    let a = 2;\n}\n",
    );

    let output = check(&staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("min_assertions"),
        "unattributed units must be exempt from min_assertions; stdout: {stdout}"
    );
}

#[test]
fn test_check_accepts_test_role_when_assertion_present() {
    let (_dir, staged) = write_temp_file(
        "tests/test_has_assertion.py",
        "def test_has_assertion():\n    assert True\n",
    );

    let output = check(&staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("min_assertions"), "stdout: {stdout}");
}

#[test]
fn test_check_assert_cmd_chain_counts_each_link() {
    // pretender-jg9y gap 1: a 3-link assert_cmd-style chain satisfies
    // min_assertions because every link counts.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/rust_assert_chain.rs"),
    )
    .unwrap_or_else(|e| panic!("read chain fixture: {e}"));
    let (dir, staged) = write_temp_file("tests/chain_test.rs", &source);

    let output = check_in(&dir, &staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("min_assertions"),
        "a 3-link assertion chain must satisfy min_assertions; stdout: {stdout}"
    );
}
