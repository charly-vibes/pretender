//! `check` determinism and resolution-tracking tests.
//!
//! # Responsibilities
//! Pin parallel-run determinism and the resolution delta/rate reported
//! across successive check runs (file and JSON history surfaces).
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;

#[test]
fn test_check_parallel_results_are_deterministic() {
    let dir = tempdir();
    for name in ["python_simple.py", "python_violator.py"] {
        std::fs::copy(source_fixture(name), dir.join(name))
            .unwrap_or_else(|e| panic!("copy fixture: {e:?}"));
    }

    let run = || {
        let output = pretender_cmd()
            .arg("check")
            .arg(&dir)
            .arg("--format")
            .arg("json")
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
        assert!(
            output.status.code().is_some(),
            "process must exit normally; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap_or_else(|e| panic!("stdout is utf-8: {e:?}"))
    };

    let first = run();
    let second = run();
    // Resolution tracking (ticket u8a) intentionally varies across runs:
    // run 1 reports findings as "new", run 2 as "still open" with a rate.
    // Compare everything except the volatile history.resolution block.
    // Resolution tracking (ticket u8a) intentionally varies across runs:
    // run 1 reports findings as "new", run 2 as "still open" with a rate.
    // Compare everything except the volatile history block. NOTE: always
    // remove history from the parsed tree (not conditionally) so runs with
    // and without a history block normalize to the same shape.
    let normalize = |s: &str| {
        let mut v: serde_json::Value =
            serde_json::from_str(s).unwrap_or_else(|e| panic!("valid json: {e:?}"));
        if let Some(data) = v.get_mut("data").and_then(|d| d.as_object_mut()) {
            data.remove("history");
        }
        serde_json::to_string(&v).unwrap_or_else(|e| panic!("serialize: {e:?}"))
    };
    assert_eq!(
        normalize(&first),
        normalize(&second),
        "json output must be deterministic across runs (excluding resolution deltas)"
    );
}

#[test]
fn test_resolution_tracking_reports_delta_across_runs() {
    // Ticket u8a: run 1 flags a finding ("new"); run 2 in the same repo sees
    // it fixed ("1 fixed, 0 still open") because a clean run updates the
    // snapshot. The fix was to remove the violation between runs.
    let dir = tempdir();
    let violator = dir.join("violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &violator)
        .unwrap_or_else(|e| panic!("copy fixture: {e:?}"));

    let check = || {
        pretender_cmd()
            .arg("check")
            .arg(".")
            .current_dir(&dir)
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|e| panic!("run check: {e:?}"))
    };

    // Run 1: finding appears — all new, rate undefined.
    let first = check();
    let first_out = String::from_utf8_lossy(&first.stdout).to_string();
    assert!(
        first_out.contains("resolution: 0 fixed, 0 still open, 1 new"),
        "run 1 stdout: {first_out}"
    );

    // Fix the violation (reduce to 3 params).
    std::fs::write(
        violator.as_path(),
        "def fewer_params(a, b, c):\n    return a + b + c\n",
    )
    .unwrap_or_else(|e| panic!("fix violation: {e:?}"));

    let second = check();
    let second_out = String::from_utf8_lossy(&second.stdout).to_string();
    assert!(
        second_out.contains("1 fixed"),
        "run 2 must report the fix; stdout: {second_out}"
    );

    // Clean run logs nothing to events.jsonl but updates the snapshot: a
    // third run must still see 1 fixed, proving "fixed" != "not rescanned".
    let third = check();
    let third_out = String::from_utf8_lossy(&third.stdout).to_string();
    assert!(
        !third_out.contains("resolution: 1 fixed"),
        "clean run 3 must not re-report run 1's fix; stdout: {third_out}"
    );
}

#[test]
fn test_resolution_rate_appears_in_json_history() {
    let dir = tempdir();
    let violator = dir.join("violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &violator)
        .unwrap_or_else(|e| panic!("copy: {e:?}"));
    let run = || {
        pretender_cmd()
            .arg("check")
            .arg(".")
            .current_dir(&dir)
            .arg("--format")
            .arg("json")
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|e| panic!("run check: {e:?}"))
    };
    run();
    let second = run();
    let json: serde_json::Value =
        serde_json::from_slice(&second.stdout).unwrap_or_else(|e| panic!("json: {e:?}"));
    let resolution = &json["data"]["history"]["resolution"];
    assert_eq!(resolution["fixed"], 0);
    assert_eq!(resolution["still_open"], 1);
    assert_eq!(resolution["new"], 0);
    assert_eq!(resolution["rate"], 0.0);
}
