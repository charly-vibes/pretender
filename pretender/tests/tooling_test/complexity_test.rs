//! Complexity-subcommand tests: argument handling and per-language metrics.
//!
//! # Responsibilities
//! Pin multi-path/directory input handling, unsupported-extension failure,
//! threshold markers, and the R/Julia/C#/Clojure complexity columns.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect/unwrap calls converted to sanctioned panic-on-failure forms.

use crate::common::*;

#[test]
fn test_complexity_command() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("python_simple.py"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains("complex_func: 6") || !stdout.contains("simple: 1") {
        panic!(
            "Output did not contain expected results.\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_complexity_unsupported_extension_fails() {
    // Contract: cli-and-config / unsupported-complexity-input-fails
    let dir = tempdir();
    let txt = dir.join("notes.txt");
    std::fs::write(&txt, "just some text\n").unwrap_or_else(|e| panic!("write txt file: {e:?}"));

    let output = pretender_cmd()
        .arg("complexity")
        .arg(&txt)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        !output.status.success(),
        "complexity on an unsupported extension must exit non-zero"
    );
}

#[test]
fn test_complexity_multi_path() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("python_simple.py"))
        .arg(source_fixture("rust_simple.rs"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Multi-path mode shows per-file headers
    assert!(
        stdout.contains("python_simple.py:"),
        "should show file header for python file; stdout: {stdout}"
    );
    assert!(
        stdout.contains("rust_simple.rs:"),
        "should show file header for rust file; stdout: {stdout}"
    );
}

#[test]
fn test_complexity_directory_path() {
    // Regression: `pretender complexity <dir>` crashed with
    // "failed to read source file" because directories were passed to
    // fs::read_to_string instead of being expanded to files.
    let dir = source_fixture("");
    let output = pretender_cmd()
        .arg("complexity")
        .arg(&dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "complexity should succeed on a directory; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("simple: 1"),
        "should have analyzed fixture files; stdout: {stdout}"
    );
}

#[test]
fn test_complexity_directory_with_non_source_files() {
    // Regression: `pretender complexity <dir>` exited 1 with
    // "missing file extension for path: <dir>/.gitignore" because the
    // directory walk collected every file, not just supported sources.
    let (dir, _src) = write_temp_file("src_simple.py", "def foo():\n    pass\n");
    std::fs::write(dir.join(".gitignore"), "target/\n")
        .unwrap_or_else(|e| panic!("write gitignore: {e:?}"));
    std::fs::write(dir.join("README.md"), "# docs\n")
        .unwrap_or_else(|e| panic!("write readme: {e:?}"));
    std::fs::create_dir_all(dir.join("sub")).unwrap_or_else(|e| panic!("mkdir: {e:?}"));
    std::fs::write(dir.join("sub/note.txt"), "hi\n")
        .unwrap_or_else(|e| panic!("write note: {e:?}"));

    let output = pretender_cmd()
        .arg("complexity")
        .arg(&dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "complexity should skip non-source files in a directory walk; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("foo"),
        "should have analyzed the python source; stdout: {stdout}"
    );
}

#[test]
fn test_complexity_shows_threshold_marker_on_violation() {
    // Write a fixture with a function that exceeds cyclomatic_max=10
    // 11 branches → cyclomatic=12
    let (dir, path) = write_temp_file(
        "high_complexity.py",
        r#"
def very_complex(x):
    if x == 1: return 1
    if x == 2: return 2
    if x == 3: return 3
    if x == 4: return 4
    if x == 5: return 5
    if x == 6: return 6
    if x == 7: return 7
    if x == 8: return 8
    if x == 9: return 9
    if x == 10: return 10
    return 0
"#,
    );
    // No pretender.toml → defaults (app cyclomatic_max=10)
    let _ = dir;

    let output = pretender_cmd()
        .arg("complexity")
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("✗"),
        "should mark function exceeding threshold with ✗; stdout: {stdout}"
    );
    assert!(
        stdout.contains("threshold: 10"),
        "should show threshold value; stdout: {stdout}"
    );
}

#[test]
fn test_r_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("r_sample.R"))
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
fn test_julia_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("julia_sample.jl"))
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
fn test_csharp_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("csharp_sample.cs"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Simple: 1"),
        "expected Simple: 1 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("WithBranch: 2"),
        "expected WithBranch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("ComplexFunc: 5"),
        "expected ComplexFunc: 5 in stdout: {stdout}"
    );
}

#[test]
fn test_clojure_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("clojure_sample.clj"))
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
        stdout.contains("with-branch: 2"),
        "expected with-branch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complex-func: 5"),
        "expected complex-func: 5 in stdout: {stdout}"
    );
}
