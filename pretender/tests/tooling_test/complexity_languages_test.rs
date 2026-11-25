//! Per-language complexity metrics: the fixture matrix across languages.
//!
//! # Responsibilities
//! Pin the complexity columns (simple/with_branch/complex_func naming and
//! values) across the Rust, JavaScript, TypeScript, Go, Java, Ruby, C, and
//! C++ fixture matrix, including nested-function branch exclusion
//! regressions.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect calls converted to the sanctioned panic-on-failure form; kept
//! apart from the argument-handling complexity tests to stay within the
//! gate's per-file size limit. The complexity subprocess invocation is
//! concentrated in `complexity_stdout`; column assertions stay in the tests.

use crate::common::*;

/// Runs `pretender complexity` on the named fixture, asserts the process
/// succeeds, and returns its stdout.
fn complexity_stdout(fixture: &str) -> String {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture(fixture))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn test_snake_case_complexity_columns_across_languages() {
    // Languages whose complexity columns are snake_case (rust, go, ruby, c,
    // cpp) all report the same three columns for their simple fixture:
    // 1 branch (with_branch 2) and 4 ifs (complex_func 5).
    let cases = [
        ("rust_simple.rs", "rust"),
        ("go_sample.go", "go"),
        ("ruby_sample.rb", "ruby"),
        ("c_sample.c", "c"),
        ("cpp_sample.cpp", "cpp"),
    ];
    for (fixture, lang) in cases {
        let stdout = complexity_stdout(fixture);
        assert!(
            stdout.contains("simple: 1"),
            "{lang}: expected simple: 1 in stdout: {stdout}"
        );
        assert!(
            stdout.contains("with_branch: 2"),
            "{lang}: expected with_branch: 2 in stdout: {stdout}"
        );
        assert!(
            stdout.contains("complex_func: 5"),
            "{lang}: expected complex_func: 5 in stdout: {stdout}"
        );
    }
}

#[test]
fn test_rust_nested_function_complexity() {
    // Nested definitions (Rust function_item) must not contribute branches
    // to the enclosing function. outer has one if; inner's if belongs to inner.
    let stdout = complexity_stdout("rust_nested.rs");

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
fn test_camel_case_complexity_columns_across_languages() {
    // Languages whose complexity columns are camelCase (javascript,
    // typescript, java) report the same column shape; TypeScript names the
    // function column after the function itself (greet: 1).
    let cases = [
        ("js_simple.js", "simple: 1", "javascript"),
        ("ts_sample.ts", "greet: 1", "typescript"),
        ("java_sample.java", "simple: 1", "java"),
    ];
    for (fixture, first_column, lang) in cases {
        let stdout = complexity_stdout(fixture);
        assert!(
            stdout.contains(first_column),
            "{lang}: expected {first_column} in stdout: {stdout}"
        );
        assert!(
            stdout.contains("withBranch: 2"),
            "{lang}: expected withBranch: 2 in stdout: {stdout}"
        );
        assert!(
            stdout.contains("complexFunc: 5"),
            "{lang}: expected complexFunc: 5 in stdout: {stdout}"
        );
    }
}

#[test]
fn test_javascript_nested_function_complexity() {
    // Nested function_declaration branches must not count toward outer.
    let stdout = complexity_stdout("js_nested.js");

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
fn test_go_nested_function_complexity() {
    // Anonymous function literal (func_literal) branches must not count
    // toward the enclosing function.
    let stdout = complexity_stdout("go_nested.go");

    assert!(
        stdout.contains("outer: 2"),
        "expected outer: 2 (closure branches excluded); got stdout: {stdout}"
    );
}
