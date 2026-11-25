//! Per-language complexity metrics: the fixture matrix across languages.
//!
//! # Responsibilities
//! Pin the complexity columns (simple/with_branch/complex_func naming and
//! values) for Rust, JavaScript, TypeScript, Go, Java, Ruby, C, and C++
//! fixtures, including nested-function branch exclusion regressions.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect calls converted to the sanctioned panic-on-failure form; kept
//! apart from the argument-handling complexity tests to stay within the
//! gate's per-file size limit.

use crate::common::*;

#[test]
fn test_rust_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("rust_simple.rs"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "expected simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("with_branch: 2"),
        "expected with_branch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complex_func: 5"),
        "expected complex_func: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_rust_nested_function_complexity() {
    // Nested definitions (Rust function_item) must not contribute branches
    // to the enclosing function. outer has one if; inner's if belongs to inner.
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("rust_nested.rs"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("outer: 2"),
        "expected outer: 2 (nested fn branches excluded); got stdout: {stdout}"
    );
    assert!(
        stdout.contains("inner: 2"),
        "expected inner: 2; got stdout: {stdout}"
    );
}

#[test]
fn test_javascript_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("js_simple.js"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "expected simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("withBranch: 2"),
        "expected withBranch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complexFunc: 5"),
        "expected complexFunc: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_javascript_nested_function_complexity() {
    // Nested function_declaration branches must not count toward outer.
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("js_nested.js"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("outer: 2"),
        "expected outer: 2 (nested fn branches excluded); got stdout: {stdout}"
    );
    assert!(
        stdout.contains("inner: 2"),
        "expected inner: 2; got stdout: {stdout}"
    );
}

#[test]
fn test_typescript_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("ts_sample.ts"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("greet: 1"),
        "expected greet: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("withBranch: 2"),
        "expected withBranch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complexFunc: 5"),
        "expected complexFunc: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_go_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("go_sample.go"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "expected simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("with_branch: 2"),
        "expected with_branch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complex_func: 5"),
        "expected complex_func: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_go_nested_function_complexity() {
    // Anonymous function literal (func_literal) branches must not count
    // toward the enclosing function.
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("go_nested.go"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("outer: 2"),
        "expected outer: 2 (closure branches excluded); got stdout: {stdout}"
    );
}

#[test]
fn test_java_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("java_sample.java"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "expected simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("withBranch: 2"),
        "expected withBranch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complexFunc: 5"),
        "expected complexFunc: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_ruby_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("ruby_sample.rb"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "expected simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("with_branch: 2"),
        "expected with_branch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complex_func: 5"),
        "expected complex_func: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_c_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("c_sample.c"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "expected simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("with_branch: 2"),
        "expected with_branch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complex_func: 5"),
        "expected complex_func: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_cpp_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("cpp_sample.cpp"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "expected simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("with_branch: 2"),
        "expected with_branch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complex_func: 5"),
        "expected complex_func: 5 in stdout: {stdout}"
    );
}
