use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_complexity_command() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // If we are in the workspace root, we need to go into the pretender package or target/debug
    // Actually, cargo test runs with CARGO_MANIFEST_DIR set to the package directory.

    let bin = root.join("../target/debug/pretender");
    let fixture = root.join("../tests/fixtures/python_simple.py");

    let output = Command::new(bin)
        .arg("complexity")
        .arg(fixture)
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
