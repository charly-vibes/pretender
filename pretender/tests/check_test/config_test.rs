//! `check` configuration and language-coverage warning tests.
//!
//! # Responsibilities
//! Pin warning emission for missing/absent config, empty-config fallback to
//! defaults, unsupported-language and supported-language (clojure) path
//! warnings, raw-identifier rust parsing, and the csharp fixture path.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;
use std::path::{Path, PathBuf};

/// Writes the pretender-mlw `raw` repro (`repro.rs` plus a minimal config)
/// into `dir` and returns the source path.
fn write_raw_repro(dir: &Path) -> std::path::PathBuf {
    let source = dir.join("repro.rs");
    std::fs::write(
        &source,
        "fn f(x: &str) -> usize {\n    let raw = x.to_string();\n    raw.len() + (&raw).len()\n}\n",
    )
    .unwrap_or_else(|e| panic!("write repro.rs: {e:?}"));
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));
    source
}

#[test]
fn test_check_rust_raw_local_no_parse_error_warning() {
    // pretender-mlw: a local named `raw` referenced as `&raw` collided with
    // the &raw const/mut expression lookahead in tree-sitter-rust 0.23.3,
    // producing a false "Parse errors detected" warning (and, worse, empty
    // metrics — the engine early-returns on has_error). Valid Rust must
    // parse cleanly.
    let dir = tempdir();
    let source = write_raw_repro(&dir);

    let output = check_in(&dir, &source)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "check should succeed; stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("Parse errors detected") && !stderr.contains("Parse errors detected"),
        "valid Rust with a local named 'raw' must not warn; stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn test_check_rust_raw_local_computes_metrics() {
    // Second half of the pretender-mlw proof: metrics must actually
    // compute — engine.rs early-returns empty units on has_error, so the
    // warning's absence alone doesn't prove units parsed.
    // `complexity` lists per-function scores.
    let dir = tempdir();
    let source = write_raw_repro(&dir);

    let cx = pretender_cmd()
        .arg("complexity")
        .arg(&source)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("run complexity: {e:?}"));
    let cx_out = String::from_utf8_lossy(&cx.stdout);
    assert!(
        cx_out.contains("f: "),
        "metrics should compute (function f listed); stdout: {cx_out}"
    );
}

#[test]
fn test_check_warns_when_no_config() {
    let dir = tempdir();
    let source = dir.join("hello.py");
    std::fs::write(&source, "def hello():\n    pass\n")
        .unwrap_or_else(|e| panic!("write source: {e:?}"));

    let output = check_in(&dir, &source)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no pretender.toml found"),
        "expected warning about missing config; stderr: {stderr}"
    );
    assert!(
        stderr.contains("pretender init"),
        "expected suggestion to run pretender init; stderr: {stderr}"
    );
}

#[test]
fn test_check_no_warning_when_config_present() {
    let dir = tempdir();
    let source = dir.join("hello.py");
    std::fs::write(&source, "def hello():\n    pass\n")
        .unwrap_or_else(|e| panic!("write source: {e:?}"));
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));

    let output = check_in(&dir, &source)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no pretender.toml found"),
        "unexpected config warning when toml is present; stderr: {stderr}"
    );
}

#[test]
fn test_empty_config_file_falls_back_to_defaults_for_check() {
    // EDGE-001: a present-but-empty pretender.toml must not break `check`.
    // genesis::config rejects empty files, so load_config short-circuits
    // zero-byte files to Config::default() (doctor stays strict).
    let dir = tempdir();
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join("python_simple.py");
    std::fs::copy(&src, dir.join("python_simple.py"))
        .unwrap_or_else(|e| panic!("copy fixture: {e:?}"));
    std::fs::write(dir.join("pretender.toml"), "")
        .unwrap_or_else(|e| panic!("write empty config: {e:?}"));

    let output = check(&dir.join("python_simple.py"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "empty config should fall back to defaults, not fail; stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("failed to read pretender.toml"),
        "empty config should not surface a read error; got: {stderr}"
    );
}

#[test]
fn test_check_warns_on_unsupported_language_paths() {
    let dir = tempdir();
    std::fs::write(
        dir.join("main.hs"),
        "module Main where\nmain :: IO ()\nmain = putStrLn \"hi\"\n",
    )
    .unwrap_or_else(|e| panic!("write haskell file: {e:?}"));

    let output = pretender_cmd()
        .arg("check")
        .arg(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    // Should still exit 0 (it's a warning, not a hard failure)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no files matched a supported grammar"),
        "expected warning about no supported grammar; stderr: {stderr}"
    );
    assert!(
        stderr.contains("warning:"),
        "warning should appear on stderr; stderr: {stderr}"
    );
}

#[test]
fn test_check_no_warning_when_supported_language_found() {
    let dir = tempdir();
    std::fs::write(dir.join("main.clj"), "(ns my-app)\n")
        .unwrap_or_else(|e| panic!("write clojure file: {e:?}"));
    std::fs::write(dir.join("main.py"), "def hello(): pass\n")
        .unwrap_or_else(|e| panic!("write python file: {e:?}"));

    let output = pretender_cmd()
        .arg("check")
        .arg(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no files matched a supported grammar"),
        "should not warn when supported files are present; stderr: {stderr}"
    );
}

#[test]
fn test_csharp_check() {
    let (_dir, staged) = stage_fixture("csharp_sample.cs");

    let output = check(&staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("csharp_sample.cs"),
        "expected csharp_sample.cs in output; stdout: {stdout}"
    );
}
