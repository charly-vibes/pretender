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

fn write_temp_file(relative: &str, source: &str) -> (PathBuf, PathBuf) {
    let dir = tempdir();
    let dest = dir.join(relative);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs");
    }
    std::fs::write(&dest, source).expect("write temp source");
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

fn check_in(dir: &Path, path: &Path) -> Command {
    let mut cmd = check(path);
    cmd.current_dir(dir);
    cmd
}

fn report_in(dir: &Path) -> Command {
    let mut cmd = Command::new(pretender_bin());
    cmd.arg("report").current_dir(dir).env("NO_COLOR", "1");
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
fn test_check_tiered_mode_exits_zero_on_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged).output().expect("failed to execute process");

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 in default tiered mode; stdout: {} stderr: {}",
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
    assert!(stdout.contains("✗"), "stdout: {stdout}");
    assert!(stdout.contains("python_violator.py"), "stdout: {stdout}");
    assert!(
        stdout.contains("VIOLATION"),
        "human output should mark threshold violations; got: {stdout}"
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
fn test_check_human_output_reports_cognitive_violations() {
    let (_dir, staged) = stage_fixture("python_cognitive.py");

    let output = check(&staged).output().expect("failed to execute process");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("deeply_nested"), "stdout: {stdout}");
    assert!(stdout.contains("cognitive"), "stdout: {stdout}");
}

#[test]
fn test_check_reports_min_assertions_for_test_role() {
    let (_dir, staged) = write_temp_file(
        "tests/test_no_assertions.py",
        "def test_missing_assertion():\n    helper()\n",
    );

    let output = check(&staged).output().expect("failed to execute process");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test_missing_assertion"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("min_assertions"), "stdout: {stdout}");
}

#[test]
fn test_check_accepts_test_role_when_assertion_present() {
    let (_dir, staged) = write_temp_file(
        "tests/test_has_assertion.py",
        "def test_has_assertion():\n    assert True\n",
    );

    let output = check(&staged).output().expect("failed to execute process");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("min_assertions"), "stdout: {stdout}");
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
fn test_check_gate_mode_from_config_fails_on_violation() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &staged).expect("copy fixture");
    std::fs::write(dir.join("pretender.toml"), "[pretender]\nmode = \"gate\"\n")
        .expect("write config");

    let output = check_in(&dir, &staged)
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_check_tiered_human_output_highlights_yellow_band() {
    let dir = tempdir();
    let staged = dir.join("yellow.py");
    std::fs::write(
        dir.join("pretender.toml"),
        "[thresholds]\ncyclomatic_max = 99\ncognitive_max = 99\n",
    )
    .expect("write config");
    std::fs::write(
        &staged,
        "def yellow_band(value):\n    if value > 0:\n        pass\n    if value > 1:\n        pass\n    if value > 2:\n        pass\n    if value > 3:\n        pass\n    if value > 4:\n        pass\n    if value > 5:\n        pass\n    if value > 6:\n        pass\n    if value > 7:\n        pass\n    if value > 8:\n        pass\n    if value > 9:\n        pass\n    if value > 10:\n        pass\n    return value\n",
    )
    .expect("write source");

    let output = check_in(&dir, &staged)
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("⚠"), "stdout: {stdout}");
    assert!(
        stdout.contains("yellow_band(): cyclomatic 12 (yellow)"),
        "stdout: {stdout}"
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
fn test_check_sarif_output_structure() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged)
        .arg("--format")
        .arg("sarif")
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success() || output.status.code() == Some(1),
        "sarif format should not exit with error 2; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let sarif: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid SARIF JSON");

    assert_eq!(
        sarif["version"].as_str(),
        Some("2.1.0"),
        "version must be 2.1.0"
    );
    assert!(sarif["$schema"].as_str().is_some(), "must have $schema");

    let runs = sarif["runs"].as_array().expect("must have runs array");
    assert_eq!(runs.len(), 1, "must have exactly one run");

    let run = &runs[0];
    assert_eq!(
        run["tool"]["driver"]["name"].as_str(),
        Some("pretender"),
        "tool.driver.name must be 'pretender'"
    );

    let rules = run["tool"]["driver"]["rules"]
        .as_array()
        .expect("must have rules array");
    assert!(
        !rules.is_empty(),
        "rules must not be empty when violations exist"
    );
    for rule in rules {
        assert!(rule["id"].as_str().is_some(), "each rule must have an id");
    }

    let results = run["results"].as_array().expect("must have results array");
    assert!(
        !results.is_empty(),
        "results must not be empty for a file with violations"
    );
    for result in results {
        assert!(
            result["ruleId"].as_str().is_some(),
            "each result must have ruleId"
        );
        assert!(
            result["message"]["text"].as_str().is_some(),
            "each result must have message.text"
        );
        let locations = result["locations"]
            .as_array()
            .expect("each result must have locations");
        assert!(
            !locations.is_empty(),
            "each result must have at least one location"
        );
        let phys = &locations[0]["physicalLocation"];
        assert!(
            phys["artifactLocation"]["uri"].as_str().is_some(),
            "must have uri"
        );
        assert!(
            phys["region"]["startLine"].as_i64().is_some(),
            "must have startLine"
        );
    }
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
fn test_rust_complexity() {
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("rust_simple.rs"))
        .output()
        .expect("failed to execute process");

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
fn test_javascript_complexity() {
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("js_simple.js"))
        .output()
        .expect("failed to execute process");

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
fn test_typescript_complexity() {
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("ts_sample.ts"))
        .output()
        .expect("failed to execute process");

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

#[test]
fn test_report_markdown_reads_last_json_check() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &staged).expect("copy fixture");

    let check_output = check_in(&dir, &staged)
        .arg("--format")
        .arg("json")
        .output()
        .expect("run json check");
    assert!(
        check_output.status.success(),
        "json check failed: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    let output = report_in(&dir)
        .arg("--format")
        .arg("markdown")
        .output()
        .expect("run report");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Pretender report"), "stdout: {stdout}");
    assert!(stdout.contains("python_violator.py"), "stdout: {stdout}");
    assert!(stdout.contains("too_many_params"), "stdout: {stdout}");
    assert!(stdout.contains("cyclomatic"), "stdout: {stdout}");
}

#[test]
fn test_report_html_writes_output_file() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    let out_path = dir.join("pretender-report.html");
    std::fs::copy(source_fixture("python_violator.py"), &staged).expect("copy fixture");

    let check_output = check_in(&dir, &staged)
        .arg("--format")
        .arg("json")
        .output()
        .expect("run json check");
    assert!(
        check_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    let output = report_in(&dir)
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out_path)
        .output()
        .expect("run report");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let html = std::fs::read_to_string(&out_path).expect("html report exists");
    assert!(html.contains("<!doctype html>"), "html: {html}");
    assert!(html.contains("python_violator.py"), "html: {html}");
    assert!(html.contains("too_many_params"), "html: {html}");
}

#[test]
fn test_report_fails_without_cached_json() {
    let dir = tempdir();

    let output = report_in(&dir).output().expect("run report");

    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to read last check report"),
        "stderr: {stderr}"
    );
}

#[test]
fn test_smell_call_weights_elevate_abc() {
    let (_dir, path) = stage_fixture("python_smell_calls.py");
    let output = Command::new(pretender_bin())
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .env("NO_COLOR", "1")
        .output()
        .expect("run pretender");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid json: {e}\nstdout: {stdout}"));
    let units = json["files"][0]["units"].as_array().expect("units array");
    let abc_for = |name: &str| -> f64 {
        units
            .iter()
            .find(|u| u["name"] == name)
            .unwrap_or_else(|| panic!("unit {name} not found"))["metrics"]["abc"]
            .as_f64()
            .unwrap_or_else(|| panic!("abc metric for {name}"))
    };
    // smell_eval: A=1, B=0, C=5 (eval weight=5.0) → sqrt(26) ≈ 5.099
    let eval_abc = abc_for("smell_eval");
    assert!(
        (eval_abc - 5.099).abs() < 0.01,
        "smell_eval abc={eval_abc}, expected ≈5.099 (sqrt(26))"
    );
    // smell_exec: A=0, B=0, C=5 (exec weight=5.0) → 5.0
    let exec_abc = abc_for("smell_exec");
    assert!(
        (exec_abc - 5.0).abs() < 0.01,
        "smell_exec abc={exec_abc}, expected 5.0"
    );
    // smell_compile: A=1, B=0, C=3 (compile weight=3.0) → sqrt(10) ≈ 3.162
    let compile_abc = abc_for("smell_compile");
    assert!(
        (compile_abc - 3.162).abs() < 0.01,
        "smell_compile abc={compile_abc}, expected ≈3.162 (sqrt(10))"
    );
}

#[test]
fn test_go_complexity() {
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("go_sample.go"))
        .output()
        .expect("failed to execute process");

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
fn test_java_complexity() {
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("java_sample.java"))
        .output()
        .expect("failed to execute process");

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
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("ruby_sample.rb"))
        .output()
        .expect("failed to execute process");

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
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("c_sample.c"))
        .output()
        .expect("failed to execute process");

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
    let output = Command::new(pretender_bin())
        .arg("complexity")
        .arg(source_fixture("cpp_sample.cpp"))
        .output()
        .expect("failed to execute process");

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
