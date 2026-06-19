use std::path::{Path, PathBuf};
use std::process::Command;

fn git_init(dir: &Path) {
    assert!(Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("git init")
        .status
        .success());
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .expect("git config name");
}

fn git_add(dir: &Path, path: &Path) {
    assert!(Command::new("git")
        .arg("add")
        .arg(path)
        .current_dir(dir)
        .output()
        .expect("git add")
        .status
        .success());
}

fn git_commit(dir: &Path, message: &str) {
    assert!(Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .expect("git commit")
        .status
        .success());
}

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

fn ci_generate_in(dir: &Path, provider: &str) -> Command {
    let mut cmd = Command::new(pretender_bin());
    cmd.arg("ci")
        .arg("generate")
        .arg(provider)
        .current_dir(dir)
        .env("NO_COLOR", "1");
    cmd
}

fn hooks_in(dir: &Path, action: &str) -> Command {
    let mut cmd = Command::new(pretender_bin());
    cmd.arg("hooks")
        .arg(action)
        .current_dir(dir)
        .env("NO_COLOR", "1");
    cmd
}

fn init_in(dir: &Path) -> Command {
    let mut cmd = Command::new(pretender_bin());
    cmd.arg("init").current_dir(dir).env("NO_COLOR", "1");
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
fn test_mutation_dry_run_python() {
    let output = Command::new(pretender_bin())
        .args([
            "mutation",
            "--dry-run",
            source_fixture("python_simple.py").to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "dry-run should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("mutmut"),
        "dry-run should name the tool; got: {stdout}"
    );
    assert!(
        stdout.contains("Planned mutants") || stdout.contains("No mutation sites"),
        "dry-run should list planned mutants; got: {stdout}"
    );
}

#[test]
fn test_stub_subcommands_exit_two() {
    for cmd in [vec!["plugins", "list"]] {
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
fn test_explain_known_metric_prints_doc() {
    let output = Command::new(pretender_bin())
        .args(["explain", "cyclomatic"])
        .output()
        .expect("failed to execute process");

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
    let output = Command::new(pretender_bin())
        .args(["explain", "not_a_real_metric"])
        .output()
        .expect("failed to execute process");

    assert_ne!(output.status.code(), Some(0), "unknown metric should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown metric") || stderr.contains("cyclomatic"),
        "stderr should name the error and list available metrics; got: {stderr}",
    );
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
fn test_init_defaults_writes_config_with_mode_override() {
    let dir = tempdir();

    let output = init_in(&dir)
        .arg("--defaults")
        .arg("--mode")
        .arg("gate")
        .output()
        .expect("run init");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = std::fs::read_to_string(dir.join("pretender.toml")).expect("config exists");
    assert!(config.contains("mode = \"gate\""), "config: {config}");
    assert!(
        config.contains("languages = [\"auto\"]"),
        "config: {config}"
    );
    assert!(config.contains("[roles.test]"), "config: {config}");
}

#[test]
fn test_init_interactive_can_install_hook_and_ci() {
    let dir = tempdir();
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("git hooks dir");

    let output = init_in(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child
                .stdin
                .as_mut()
                .expect("stdin")
                .write_all(b"guidance\npython,rust\ny\ny\n")?;
            child.wait_with_output()
        })
        .expect("run init interactively");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = std::fs::read_to_string(dir.join("pretender.toml")).expect("config exists");
    assert!(config.contains("mode = \"guidance\""), "config: {config}");
    assert!(
        config.contains("languages = [\"python\", \"rust\"]"),
        "config: {config}"
    );

    let hook = std::fs::read_to_string(dir.join(".git/hooks/pre-commit")).expect("hook exists");
    assert!(hook.contains("Installed by Pretender"), "hook: {hook}");
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
    );

    let workflow = std::fs::read_to_string(dir.join(".github/workflows/pretender.yml"))
        .expect("workflow exists");
    assert!(
        workflow
            .contains("cargo install --git https://github.com/charly/pretender --locked pretender"),
        "workflow: {workflow}"
    );
}

#[test]
fn test_ci_generate_github_writes_workflow() {
    let dir = tempdir();

    let output = ci_generate_in(&dir, "github")
        .output()
        .expect("run ci generator");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let workflow = std::fs::read_to_string(dir.join(".github/workflows/pretender.yml"))
        .expect("workflow exists");
    assert!(
        workflow.starts_with("name: Pretender"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("uses: actions/checkout@v4"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("uses: dtolnay/rust-toolchain@stable"),
        "workflow: {workflow}"
    );
    assert!(
        workflow
            .contains("cargo install --git https://github.com/charly/pretender --locked pretender"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("pretender check . --format=sarif --output=pretender.sarif"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("uses: github/codeql-action/upload-sarif@v3"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("pretender report --format=markdown >> $GITHUB_STEP_SUMMARY"),
        "workflow: {workflow}"
    );
}

#[test]
fn test_ci_generate_non_github_stays_stubbed() {
    let dir = tempdir();

    let output = ci_generate_in(&dir, "gitlab")
        .output()
        .expect("run ci generator");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not yet implemented"), "stderr: {stderr}");
}

#[test]
fn test_hooks_install_and_uninstall_manage_pretender_shim() {
    let dir = tempdir();
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("git hooks dir");

    let install = hooks_in(&dir, "install")
        .output()
        .expect("run hooks install");
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );

    let hook_path = dir.join(".git/hooks/pre-commit");
    let hook = std::fs::read_to_string(&hook_path).expect("hook exists");
    assert!(hook.contains("Installed by Pretender"), "hook: {hook}");
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
    );

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .expect("run hooks uninstall");
    assert!(
        uninstall.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
    assert!(!hook_path.exists(), "hook should be removed");
}

#[test]
fn test_hooks_install_creates_hooks_dir_when_absent() {
    let dir = tempdir();
    std::fs::create_dir_all(dir.join(".git")).expect("git dir");
    // .git/hooks does NOT exist

    let install = hooks_in(&dir, "install")
        .output()
        .expect("run hooks install");
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    assert!(
        dir.join(".git/hooks/pre-commit").exists(),
        "hook should exist even when .git/hooks was absent"
    );
}

#[test]
fn test_hooks_uninstall_is_noop_when_no_hook() {
    let dir = tempdir();
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("git hooks dir");
    // no hook file present

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .expect("run hooks uninstall");
    assert!(
        uninstall.status.success(),
        "uninstall with no hook should succeed silently; stderr: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
}

#[test]
fn test_hooks_uninstall_refuses_foreign_hook() {
    let dir = tempdir();
    let hooks_dir = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).expect("git hooks dir");
    std::fs::write(
        hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'custom hook'\n",
    )
    .expect("write foreign hook");

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .expect("run hooks uninstall");
    assert!(
        !uninstall.status.success(),
        "should refuse to remove a hook not installed by Pretender"
    );
    let stderr = String::from_utf8_lossy(&uninstall.stderr);
    assert!(
        stderr.contains("refusing to remove hook not installed by Pretender"),
        "stderr: {stderr}"
    );
    assert!(
        hooks_dir.join("pre-commit").exists(),
        "foreign hook should be left intact"
    );
}

#[test]
fn test_hooks_install_refuses_foreign_hook() {
    let dir = tempdir();
    let hooks_dir = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).expect("git hooks dir");
    std::fs::write(
        hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'custom hook'\n",
    )
    .expect("write foreign hook");

    let install = hooks_in(&dir, "install")
        .output()
        .expect("run hooks install");
    assert!(
        !install.status.success(),
        "should refuse to overwrite a hook not installed by Pretender"
    );
    let stderr = String::from_utf8_lossy(&install.stderr);
    assert!(
        stderr.contains("refusing to overwrite hook not installed by Pretender"),
        "stderr: {stderr}"
    );
    let hook = std::fs::read_to_string(hooks_dir.join("pre-commit")).expect("hook still exists");
    assert!(
        hook.contains("custom hook"),
        "foreign hook should be left intact: {hook}"
    );
}

#[test]
fn test_hooks_install_is_idempotent() {
    let dir = tempdir();
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("git hooks dir");

    let first = hooks_in(&dir, "install").output().expect("first install");
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = hooks_in(&dir, "install").output().expect("second install");
    assert!(
        second.status.success(),
        "reinstall over Pretender-owned hook should succeed; stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let hook = std::fs::read_to_string(dir.join(".git/hooks/pre-commit")).expect("hook exists");
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
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
fn test_report_markdown_reads_last_human_check() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &staged).expect("copy fixture");

    let check_output = check_in(&dir, &staged).output().expect("run human check");
    assert!(
        check_output.status.success(),
        "human check failed: {}",
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
}

#[test]
fn test_report_fails_without_cached_report() {
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
fn test_check_staged_only_scans_staged_files() {
    let dir = tempdir();
    git_init(&dir);

    // Commit a baseline file so HEAD exists
    let baseline = dir.join("baseline.py");
    std::fs::write(&baseline, "def baseline(): pass\n").unwrap();
    git_add(&dir, &baseline);
    git_commit(&dir, "baseline");

    // Stage file_a.py
    let file_a = dir.join("file_a.py");
    std::fs::write(&file_a, "def func_a(): pass\n").unwrap();
    git_add(&dir, &file_a);

    // Write file_b.py but do NOT stage it
    let file_b = dir.join("file_b.py");
    std::fs::write(&file_b, "def func_b(): pass\n").unwrap();

    let output = Command::new(pretender_bin())
        .args(["check", ".", "--staged"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("file_a.py"),
        "expected file_a.py in stdout: {stdout}"
    );
    assert!(
        !stdout.contains("file_b.py"),
        "unexpected file_b.py in stdout (not staged): {stdout}"
    );
}

#[test]
fn test_check_staged_empty_staging_area_exits_success() {
    let dir = tempdir();
    git_init(&dir);

    // Commit a file so HEAD exists
    let f = dir.join("committed.py");
    std::fs::write(&f, "def f(): pass\n").unwrap();
    git_add(&dir, &f);
    git_commit(&dir, "init");

    // Nothing staged — check should succeed with no file output
    let output = Command::new(pretender_bin())
        .args(["check", ".", "--staged"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_staged_first_commit_no_head() {
    let dir = tempdir();
    git_init(&dir);

    // No prior commits — stage one file
    let f = dir.join("first.py");
    std::fs::write(&f, "def first(): pass\n").unwrap();
    git_add(&dir, &f);

    let output = Command::new(pretender_bin())
        .args(["check", ".", "--staged"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("first.py"),
        "expected first.py in stdout: {stdout}"
    );
}

#[test]
fn test_check_diff_only_filters_to_changed_files() {
    let dir = tempdir();
    git_init(&dir);

    // First commit: file_old.py
    let file_old = dir.join("file_old.py");
    std::fs::write(&file_old, "def old(): pass\n").unwrap();
    git_add(&dir, &file_old);
    git_commit(&dir, "first commit");

    // Second commit (HEAD): add file_new.py
    let file_new = dir.join("file_new.py");
    std::fs::write(&file_new, "def new_func(): pass\n").unwrap();
    git_add(&dir, &file_new);
    git_commit(&dir, "second commit");

    // --diff-only --diff-base=HEAD~1 should only show file_new.py
    let output = Command::new(pretender_bin())
        .args(["check", ".", "--diff-only", "--diff-base=HEAD~1"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("file_new.py"),
        "expected file_new.py in stdout: {stdout}"
    );
    assert!(
        !stdout.contains("file_old.py"),
        "unexpected file_old.py in stdout (unchanged): {stdout}"
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

const RUFF_PLUGIN_TOML: &str = r#"
name = "ruff"
extensions = [".py"]
command = ["ruff", "check", "--output-format=json", "--select=E501", "{files}"]
parser = "json"

[mapping]
path = "filename"
line = "location.row"
message = "message"
code = "code"
"#;

/// The python_simple.py fixture has a 112-char comment on line 1, which triggers
/// E501 (line too long > 79) when ruff is run with --select=E501.
#[test]
fn test_external_plugin_ruff_json_findings() {
    if std::process::Command::new("ruff")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping: ruff not in PATH");
        return;
    }

    let metrics_dir = tempdir();
    std::fs::write(metrics_dir.join("ruff.toml"), RUFF_PLUGIN_TOML).expect("write ruff.toml");

    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .env("PRETENDER_METRICS_DIR", &metrics_dir)
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid json output");

    let files = json["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    let external = files[0]["external_findings"]
        .as_array()
        .expect("external_findings present when ruff fires");
    assert!(
        !external.is_empty(),
        "expected at least one ruff finding; full output: {json}"
    );
    assert_eq!(
        external[0]["source"].as_str(),
        Some("ruff"),
        "finding source should be 'ruff'"
    );
    assert_eq!(
        external[0]["code"].as_str(),
        Some("E501"),
        "finding code should be 'E501'"
    );
}

#[test]
fn test_check_skips_binary_files() {
    let dir = tempdir();
    // Write a binary file (null bytes are not valid UTF-8)
    let binary = dir.join("artifact.bin");
    std::fs::write(&binary, b"\x00\x01\x02\x03\xff\xfe").expect("write binary file");

    let output = check(&binary)
        .output()
        .expect("failed to execute process");

    assert_eq!(
        output.status.code(),
        Some(0),
        "check should exit 0 when given a binary file; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_skips_binary_files_in_directory() {
    let dir = tempdir();
    // Write a source file alongside a binary file
    let source = dir.join("hello.py");
    std::fs::write(&source, "def hello():\n    pass\n").expect("write source");
    let binary = dir.join("artifact.bin");
    std::fs::write(&binary, b"\x00\x01\x02\x03\xff\xfe").expect("write binary file");

    let output = check(&dir).output().expect("failed to execute process");

    assert_eq!(
        output.status.code(),
        Some(0),
        "check should exit 0 when directory contains binary files; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
