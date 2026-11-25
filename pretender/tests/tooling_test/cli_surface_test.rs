//! CLI surface and feedback tests: stubs, typos, version, explain, feedback.
//!
//! # Responsibilities
//! Pin not-yet-implemented subcommand exits (code 2), typo suggestions,
//! `--version`/`-V` output, `explain` docs and unknown-metric failure, the
//! non-zero-exit feedback footer, and the CORR-001 full-argv error scratch.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect/unwrap calls converted to sanctioned panic-on-failure forms;
//! version tests kept with the other CLI-surface behavior.

use crate::common::*;

#[test]
fn test_stub_subcommands_exit_two() {
    let cmd = vec!["plugins", "list"];
    let output = pretender_cmd()
        .args(&cmd)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(2),
        "{cmd:?} should exit 2 (not yet implemented); stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not yet implemented"),
        "{cmd:?} stderr should explain status; got: {stderr}",
    );
}

#[test]
fn test_typo_suggestion_for_misspelled_command() {
    let output = pretender_cmd()
        .args(["complxity"])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(!output.status.success(), "typo should exit non-zero");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Did you mean") && stderr.contains("complexity"),
        "stderr should contain 'Did you mean complexity?'; got: {stderr}"
    );
}

#[test]
fn test_feedback_dry_run_prints_body_and_gh_line() {
    // Isolate the error scratch in a private cache dir: tests run in parallel
    // and share the default ~/.cache/pretender/errors.jsonl otherwise, so a
    // concurrent write can clobber the record this test just created.
    let cache = tempdir().join("cache");
    std::fs::create_dir_all(&cache).unwrap_or_else(|e| panic!("create cache dir: {e:?}"));

    // First simulate an error so --from-last-error has data
    let _ = pretender_cmd()
        .args(["explain", "not_a_real_metric"])
        .env("XDG_CACHE_HOME", &cache)
        .output();

    let output = pretender_cmd()
        .args(["feedback", "bug", "--dry-run", "--from-last-error"])
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("gh issue create"),
        "dry run should print gh command; got stderr: {stderr}"
    );
}

#[test]
fn test_non_zero_exit_shows_feedback_footer() {
    let output = pretender_cmd()
        .args(["explain", "not_a_real_metric"])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        !output.status.success(),
        "unknown metric should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Feedback") || stderr.contains("feedback bug"),
        "stderr should suggest using feedback; got: {stderr}"
    );
}

#[test]
fn test_error_scratch_records_full_argv() {
    // CORR-001: `feedback --from-last-error` surfaces the failing command,
    // so the scratch record must capture the full argv, not just the tool
    // name. ErrorSink's built-in scratch only records argv=[tool_name];
    // pretender overrides it with std::env::args().
    let cache = tempdir().join("cache");
    std::fs::create_dir_all(&cache).unwrap_or_else(|e| panic!("create cache dir: {e:?}"));

    let output = pretender_cmd()
        .args(["explain", "not_a_real_metric"])
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    assert!(!output.status.success(), "should exit non-zero");

    let scratch = cache.join("pretender").join("errors.jsonl");
    let content = std::fs::read_to_string(&scratch)
        .unwrap_or_else(|e| panic!("scratch file should exist at {}: {e}", scratch.display()));
    let parsed: serde_json::Value = serde_json::from_str(content.trim_end_matches('\n'))
        .unwrap_or_else(|e| panic!("scratch should be one JSON object: {e}\n{content}"));
    let argv = parsed["argv"]
        .as_array()
        .unwrap_or_else(|| panic!("scratch should have argv array"))
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect::<Vec<_>>();
    assert!(
        argv.iter().any(|a| a == "explain"),
        "scratch argv should include the subcommand; got: {argv:?}"
    );
    assert!(
        argv.iter().any(|a| a == "not_a_real_metric"),
        "scratch argv should include the metric arg; got: {argv:?}"
    );
    // Sanity: more than just the tool name (the regression we fixed).
    assert!(
        argv.len() > 1,
        "scratch argv should capture the full command, not just the tool name; got: {argv:?}"
    );
}

#[test]
fn test_explain_known_metric_prints_doc() {
    let output = pretender_cmd()
        .args(["explain", "cyclomatic"])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "explain cyclomatic should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cyclomatic"),
        "output should include metric name"
    );
    assert!(stdout.contains("McCabe"), "output should cite McCabe");
    assert!(
        stdout.contains("10"),
        "output should mention default threshold"
    );
}

#[test]
fn test_explain_unknown_metric_exits_nonzero() {
    let output = pretender_cmd()
        .args(["explain", "not_a_real_metric"])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_ne!(output.status.code(), Some(0), "unknown metric should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown metric") || stderr.contains("cyclomatic"),
        "stderr should name the error and list available metrics; got: {stderr}",
    );
}

// No clap version set yet; move to a version-based test once #[command(version)] is added
#[test]
fn test_version_flag_works() {
    let output = pretender_cmd()
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "--version should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("pretender"),
        "--version should print 'pretender'; got: {stdout}"
    );
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "--version should include version number; got: {stdout}"
    );
}

#[test]
fn test_version_short_flag_works() {
    let output = pretender_cmd()
        .arg("-V")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "-V should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("pretender"),
        "-V should print 'pretender'; got: {stdout}"
    );
}
