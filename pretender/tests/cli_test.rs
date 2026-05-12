use std::path::{Path, PathBuf};
use std::process::Command;

fn pretender_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/pretender")
}

fn source_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name)
}

// Copy a fixture into a fresh temp dir so role detection treats the file
// as the default `app` role rather than `test` (which would trip the
// stricter test thresholds against simple fixtures).
fn stage_fixture(name: &str) -> (PathBuf, PathBuf) {
    let dir = tempdir();
    let dest = dir.join(name);
    std::fs::copy(source_fixture(name), &dest).expect("copy fixture");
    (dir, dest)
}

fn tempdir() -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("pretender-cli-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create tempdir");
    dir
}

fn check(path: &Path) -> Command {
    let mut cmd = Command::new(pretender_bin());
    cmd.arg("check").arg(path).env("NO_COLOR", "1");
    cmd
}

#[test]
fn test_complexity_command() {
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("python_simple.py"))
        .output()
        .expect("failed to execute process");

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
fn test_check_command_human_output() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged).output().expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("python_simple.py"), "stdout: {stdout}");
    assert!(stdout.contains("complex_func"), "stdout: {stdout}");
    assert!(stdout.contains("cyclomatic"), "stdout: {stdout}");
}

#[test]
fn test_check_command_json_output() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");

    assert_eq!(json["files"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        json["files"][0]["path"].as_str(),
        Some(staged.to_string_lossy().as_ref())
    );
    assert_eq!(
        json["files"][0]["units"][0]["name"].as_str(),
        Some("simple")
    );
    assert_eq!(
        json["files"][0]["units"][2]["metrics"]["cyclomatic"].as_u64(),
        Some(6)
    );
}

#[test]
fn test_check_exits_zero_when_clean() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged).output().expect("failed to execute process");

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 on clean fixture; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_exits_nonzero_on_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged).output().expect("failed to execute process");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when violations exist; stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_output_flag_writes_to_file() {
    let (_dir, staged) = stage_fixture("python_simple.py");
    let out_path = staged.with_file_name("report.json");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(&out_path)
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty when --output is set; got: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let written = std::fs::read_to_string(&out_path).expect("report file should exist");
    let json: serde_json::Value =
        serde_json::from_str(&written).expect("report file should be valid json");
    assert_eq!(json["files"].as_array().map(Vec::len), Some(1));
}

#[test]
fn test_check_human_output_to_file_is_uncolored() {
    let (_dir, staged) = stage_fixture("python_simple.py");
    let out_path = staged.with_file_name("report.txt");

    let output = check(&staged)
        .arg("--output")
        .arg(&out_path)
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let written = std::fs::read_to_string(&out_path).expect("report file should exist");
    assert!(
        !written.contains('\u{1b}'),
        "report written to file must not contain ANSI escape sequences: {written:?}"
    );
}

#[test]
fn test_check_human_output_surfaces_violations() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged).output().expect("failed to execute process");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("VIOLATION"),
        "human output should mark violations; got: {stdout}"
    );
    assert!(
        stdout.contains("params"),
        "human output should name the violated metric; got: {stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "NO_COLOR should suppress ANSI codes; got: {stdout:?}"
    );
}

#[test]
fn test_check_guidance_mode_exits_zero_on_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged)
        .arg("--mode")
        .arg("guidance")
        .output()
        .expect("failed to execute process");

    assert_eq!(
        output.status.code(),
        Some(0),
        "guidance mode must exit 0 even when violations exist; stdout: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("VIOLATION"),
        "guidance mode still surfaces violations as annotations; got: {stdout}",
    );
}

#[test]
fn test_check_gate_mode_fails_on_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged)
        .arg("--mode")
        .arg("gate")
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_check_sarif_format_returns_not_implemented() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("sarif")
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not yet implemented"),
        "stderr should describe missing feature; got: {stderr}",
    );
}

#[test]
fn test_check_staged_flag_returns_not_implemented() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--staged")
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_stub_subcommands_exit_two() {
    for cmd in [
        vec!["init"],
        vec!["report"],
        vec!["duplication"],
        vec!["mutation"],
        vec!["hooks", "install"],
        vec!["ci", "generate", "github"],
        vec!["plugins", "list"],
        vec!["explain", "cyclomatic"],
    ] {
        let output = Command::new(pretender_bin())
            .args(&cmd)
            .output()
            .expect("failed to execute process");

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
}

#[test]
fn test_check_parallel_results_are_deterministic() {
    let dir = tempdir();
    for name in ["python_simple.py", "python_violator.py"] {
        std::fs::copy(source_fixture(name), dir.join(name)).expect("copy fixture");
    }

    let run = || {
        let output = Command::new(pretender_bin())
            .arg("check")
            .arg(&dir)
            .arg("--format")
            .arg("json")
            .env("NO_COLOR", "1")
            .output()
            .expect("failed to execute process");
        assert!(
            output.status.code().is_some(),
            "process must exit normally; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("stdout is utf-8")
    };

    let first = run();
    let second = run();
    assert_eq!(
        first, second,
        "json output must be deterministic across runs"
    );
}
