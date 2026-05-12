use std::path::PathBuf;
use std::process::Command;

fn pretender_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/pretender")
}

fn python_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/python_simple.py")
}

#[test]
fn test_complexity_command() {
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(python_fixture())
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
    let output = Command::new(pretender_bin())
        .arg("check")
        .arg(python_fixture())
        .output()
        .expect("failed to execute process");

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
    let output = Command::new(pretender_bin())
        .arg("check")
        .arg(python_fixture())
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
        Some(python_fixture().to_string_lossy().as_ref())
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
