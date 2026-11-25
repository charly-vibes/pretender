mod common;

use common::*;

use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn test_meter_gate_hook_stops_commit() {
    // Ticket 15w acceptance, executable: the meter seeds a scratch repo with
    // a violating function, installs the hook, and asserts the gate stops the
    // commit with the function name + rule in the output.
    let meter = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/meter-gate-hook.sh");
    let output = Command::new("sh")
        .arg(meter)
        .env("PRETENDER_BIN", pretender_bin())
        .env_remove("GIT_CONFIG_GLOBAL")
        .output()
        .expect("run meter script");
    assert!(
        output.status.success(),
        "meter failed; stdout+stderr: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_complexity_command() {
    let output = pretender_cmd()
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
fn test_complexity_unsupported_extension_fails() {
    // Contract: cli-and-config / unsupported-complexity-input-fails
    let dir = tempdir();
    let txt = dir.join("notes.txt");
    std::fs::write(&txt, "just some text\n").expect("write txt file");

    let output = pretender_cmd()
        .arg("complexity")
        .arg(&txt)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to execute process");

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
        .expect("failed to execute process");

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
        .expect("failed to execute process");

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
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("write gitignore");
    std::fs::write(dir.join("README.md"), "# docs\n").expect("write readme");
    std::fs::create_dir_all(dir.join("sub")).expect("mkdir");
    std::fs::write(dir.join("sub/note.txt"), "hi\n").expect("write note");

    let output = pretender_cmd()
        .arg("complexity")
        .arg(&dir)
        .output()
        .expect("failed to execute process");

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
        .expect("failed to execute process");

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
fn test_doctor_json_output_has_envelope_shape() {
    let dir = tempdir();
    git_init(&dir);

    let output = doctor_in(&dir, &["--json"]).output().expect("run doctor");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // Verify genesis envelope shape
    assert!(json.get("ok").is_some(), "envelope must have 'ok' field");
    assert_eq!(json["ok"], true, "doctor ran successfully, ok=true");
    assert_eq!(
        json["envelope_version"], "0.1",
        "envelope_version should be 0.1"
    );
    assert_eq!(
        json["envelope_kind"], "doctor",
        "envelope_kind should be 'doctor'"
    );
    assert!(
        json.get("data").is_some(),
        "envelope must have 'data' field"
    );
    // Verify existing data is preserved within the envelope
    let checks = json["data"]["checks"]
        .as_array()
        .expect("data.checks should be array");
    let config_present = checks
        .iter()
        .find(|c| c["name"] == "Config present")
        .expect("Config present check missing");
    assert_eq!(
        config_present["status"], "fail",
        "Config present should have status=fail"
    );
}

#[test]
fn test_mutation_dry_run_python() {
    let output = pretender_cmd()
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
fn test_mutation_dry_run_directory() {
    // Regression: `mutation --dry-run <dir>` failed with
    // "no supported source files found" because directories were passed
    // to detect_language() which checks file extensions.
    let dir = source_fixture("");
    let output = pretender_cmd()
        .args(["mutation", "--dry-run", dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    assert!(
        output.status.success(),
        "mutation --dry-run should succeed on a directory; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_stub_subcommands_exit_two() {
    let cmd = vec!["plugins", "list"];
    let output = pretender_cmd()
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

#[test]
fn test_typo_suggestion_for_misspelled_command() {
    let output = pretender_cmd()
        .args(["complxity"])
        .output()
        .expect("failed to execute process");

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
    std::fs::create_dir_all(&cache).expect("create cache dir");

    // First simulate an error so --from-last-error has data
    let _ = pretender_cmd()
        .args(["explain", "not_a_real_metric"])
        .env("XDG_CACHE_HOME", &cache)
        .output();

    let output = pretender_cmd()
        .args(["feedback", "bug", "--dry-run", "--from-last-error"])
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .expect("failed to execute process");

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
        .expect("failed to execute process");

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
    std::fs::create_dir_all(&cache).expect("create cache dir");

    let output = pretender_cmd()
        .args(["explain", "not_a_real_metric"])
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .expect("failed to execute process");
    assert!(!output.status.success(), "should exit non-zero");

    let scratch = cache.join("pretender").join("errors.jsonl");
    let content = std::fs::read_to_string(&scratch)
        .unwrap_or_else(|e| panic!("scratch file should exist at {}: {e}", scratch.display()));
    let parsed: serde_json::Value = serde_json::from_str(content.trim_end_matches('\n'))
        .unwrap_or_else(|e| panic!("scratch should be one JSON object: {e}\n{content}"));
    let argv = parsed["argv"]
        .as_array()
        .expect("scratch should have argv array")
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
    let output = pretender_cmd()
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
    let output = pretender_cmd()
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
fn test_rust_nested_function_complexity() {
    // Nested definitions (Rust function_item) must not contribute branches
    // to the enclosing function. outer has one if; inner's if belongs to inner.
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("rust_nested.rs"))
        .output()
        .expect("failed to execute process");

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
fn test_javascript_nested_function_complexity() {
    // Nested function_declaration branches must not count toward outer.
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("js_nested.js"))
        .output()
        .expect("failed to execute process");

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
fn test_init_defaults_flag_is_rejected() {
    let dir = tempdir();

    let output = init_in(&dir).arg("--defaults").output().expect("run init");

    assert!(
        !output.status.success(),
        "expected failure for removed --defaults flag"
    );
}

#[test]
fn test_init_non_interactive_writes_config_with_mode_override() {
    let dir = tempdir();

    let output = init_in(&dir)
        .arg("--non-interactive")
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

const FOREIGN_AGENTS: &str = "\
# AGENTS.md

<!-- WAI:START -->
Run `wai status` to orient yourself.
<!-- WAI:END -->

<!-- DONT:START -->
See .dont/ for grounded-claim workflow.
<!-- DONT:END -->
";

/// Seed an AGENTS.md carrying wai- and dont-authored blocks, run
/// `pretender init --non-interactive`, and return the AGENTS.md path.
fn seed_foreign_blocks_and_init(dir: &std::path::Path) -> std::path::PathBuf {
    // pretender-1ap: pretender owns no AGENTS.md region. WAI and DONT are
    // authored by wai and dont, so `pretender init` must not write into
    // either block, and the doctor must not report a foreign block as
    // drifted against a pretender generator it never published.
    git_init(dir);
    let agents = dir.join("AGENTS.md");
    std::fs::write(&agents, FOREIGN_AGENTS).expect("seed AGENTS.md");
    let output = init_in(dir)
        .arg("--non-interactive")
        .output()
        .expect("run init");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    agents
}

#[test]
fn test_init_leaves_foreign_managed_blocks_untouched() {
    let dir = tempdir();
    let agents = seed_foreign_blocks_and_init(&dir);
    let after_init = std::fs::read(&agents).expect("read AGENTS.md");
    assert_eq!(
        after_init,
        FOREIGN_AGENTS.as_bytes(),
        "pretender init must leave foreign managed blocks byte-identical"
    );
}

#[test]
fn test_check_and_doctor_report_no_foreign_block_drift() {
    let dir = tempdir();
    let agents = seed_foreign_blocks_and_init(&dir);

    // A real source file keeps the scan non-empty, so a clean run proves the
    // absence of block findings rather than of work.
    std::fs::write(dir.join("example.py"), "def answer():\n    return 42\n")
        .expect("write fixture");

    let check_output = check_default(&dir)
        .arg("--json")
        .output()
        .expect("run check");
    let check_text = format!(
        "{}{}",
        String::from_utf8_lossy(&check_output.stdout),
        String::from_utf8_lossy(&check_output.stderr)
    );
    assert!(
        !check_text.contains("managed_block_drift"),
        "check must not emit a managed-block drift finding: {check_text}"
    );

    let doctor_output = doctor_in(&dir, &["--json"]).output().expect("run doctor");
    let doctor_text = format!(
        "{}{}",
        String::from_utf8_lossy(&doctor_output.stdout),
        String::from_utf8_lossy(&doctor_output.stderr)
    );
    assert!(
        !doctor_text.contains("managed_block_drift"),
        "doctor must not register a managed-block drift check: {doctor_text}"
    );
    for block in ["WAI", "DONT"] {
        assert!(
            !check_text.contains(block) && !doctor_text.contains(block),
            "neither check nor doctor may name the {block} block it does not own"
        );
    }

    let after_checks = std::fs::read(&agents).expect("re-read AGENTS.md");
    assert_eq!(
        after_checks,
        FOREIGN_AGENTS.as_bytes(),
        "check/doctor must not rewrite foreign managed blocks"
    );
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
                // mode, preset (added by update-app-threshold-defaults),
                // languages, hook, workflow
                .write_all(b"guidance\n\npython,rust\ny\ny\n")?;
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
        workflow.contains(
            "cargo install --git https://github.com/charly-vibes/pretender --locked pretender"
        ),
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
        workflow.contains(
            "cargo install --git https://github.com/charly-vibes/pretender --locked pretender"
        ),
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
fn test_hooks_install_pre_push_installs_pre_push_hook() {
    let dir = tempdir();
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("git hooks dir");

    let install = hooks_in_named(&dir, ("install", "pre-push"))
        .output()
        .expect("run hooks install pre-push");
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );

    let hook_path = dir.join(".git/hooks/pre-push");
    let hook = std::fs::read_to_string(&hook_path).expect("pre-push hook exists");
    assert!(hook.contains("Installed by Pretender"), "hook: {hook}");
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
    );
    assert!(
        !dir.join(".git/hooks/pre-commit").exists(),
        "pre-commit untouched"
    );

    let uninstall = hooks_in_named(&dir, ("uninstall", "pre-push"))
        .output()
        .expect("run hooks uninstall pre-push");
    assert!(
        uninstall.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
    assert!(!hook_path.exists(), "pre-push hook should be removed");
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
fn test_hooks_install_from_subdirectory() {
    // Hooks must be installed at the repo root, not the CWD.
    let dir = tempdir();
    std::fs::create_dir_all(dir.join(".git/hooks")).expect("git hooks dir");
    let subdir = dir.join("src/sub/deep");
    std::fs::create_dir_all(&subdir).expect("create subdir");

    let install = hooks_in(&subdir, "install")
        .output()
        .expect("run hooks install from subdir");
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    // Hook must be at repo root, not in the subdir
    assert!(
        dir.join(".git/hooks/pre-commit").exists(),
        "hook should be at repo root, not subdir"
    );
    assert!(
        !subdir.join(".git/hooks/pre-commit").exists(),
        "hook should NOT be created in subdirectory"
    );
}

#[test]
fn test_hooks_uninstall_outside_repo_fails_cleanly() {
    // Running hooks outside a git repo must fail clearly.
    let dir = tempdir();

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .expect("run hooks uninstall outside repo");
    assert!(
        !uninstall.status.success(),
        "should fail outside a git repo"
    );
    let stderr = String::from_utf8_lossy(&uninstall.stderr);
    assert!(
        stderr.contains("git") || stderr.contains("repository"),
        "error should mention git/repository; got: {stderr}"
    );
}

#[test]
fn test_smell_call_weights_elevate_abc() {
    let (_dir, path) = stage_fixture("python_smell_calls.py");
    let output = pretender_cmd()
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
    let units = json["data"]["files"][0]["units"]
        .as_array()
        .expect("units array");
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
    let output = pretender_cmd()
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
fn test_go_nested_function_complexity() {
    // Anonymous function literal (func_literal) branches must not count
    // toward the enclosing function.
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("go_nested.go"))
        .output()
        .expect("failed to execute process");

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
    let output = pretender_cmd()
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
    let output = pretender_cmd()
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
    let output = pretender_cmd()
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

    let files = json["data"]["files"].as_array().expect("files array");
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

// No clap version set yet; move to a version-based test once #[command(version)] is added
#[test]
fn test_version_flag_works() {
    let output = pretender_cmd()
        .arg("--version")
        .output()
        .expect("failed to execute process");

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
        .expect("failed to execute process");

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

// Redirect git's global config to a neutral empty file so doctor's
// core.hooksPath checks are hermetic regardless of the developer machine's
// real ~/.gitconfig (some machines carry a global hooksPath shim).
fn neutral_git_config() -> &'static std::path::Path {
    neutral_git_config_global()
}

fn doctor_in(dir: &Path, extra_args: &[&str]) -> Command {
    let mut cmd = pretender_cmd();
    cmd.arg("doctor")
        .args(extra_args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GIT_CONFIG_GLOBAL", neutral_git_config());
    cmd
}

fn write_pretender_hook(dir: &Path) {
    let hooks_dir = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).expect("create hooks dir");
    let hook = hooks_dir.join("pre-commit");
    std::fs::write(
        &hook,
        "#!/usr/bin/env sh\n# Installed by Pretender.\nexec pretender check . --staged\n",
    )
    .expect("write hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
            .expect("chmod hook");
    }
}

#[test]
fn test_doctor_all_pass_with_valid_config_and_hook() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n").expect("write config");
    write_pretender_hook(&dir);

    let output = doctor_in(&dir, &["--human"]).output().expect("run doctor");

    assert_eq!(
        output.status.code(),
        Some(0),
        "doctor should exit 0; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("checks passed"),
        "expected summary; got: {stdout}"
    );
}

#[test]
fn test_doctor_missing_config_exits_1_with_correct_skips() {
    let dir = tempdir();
    git_init(&dir);

    let output = doctor_in(&dir, &["--human"]).output().expect("run doctor");

    assert_eq!(output.status.code(), Some(1), "should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("✗ Config present"),
        "Config present should fail; got: {stdout}"
    );
    assert!(
        stdout.contains("⚠ Config valid"),
        "Config valid should be skipped; got: {stdout}"
    );
    assert!(
        stdout.contains("⚠ Plugin manifests"),
        "Plugin manifests should be skipped; got: {stdout}"
    );
}

#[test]
fn test_doctor_json_missing_config_exits_1_with_fail_status() {
    let dir = tempdir();
    git_init(&dir);

    let output = doctor_in(&dir, &["--json"]).output().expect("run doctor");

    assert_eq!(output.status.code(), Some(1), "should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let checks = parsed["data"]["checks"]
        .as_array()
        .expect("JSON data.checks should be array");
    let config_present = checks
        .iter()
        .find(|c| c["name"] == "Config present")
        .expect("Config present check missing");
    assert_eq!(
        config_present["status"], "fail",
        "Config present should have status=fail"
    );
}

#[test]
fn test_doctor_unmanaged_hook_fails_installed_skips_executable() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n").expect("write config");
    let hooks_dir = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).expect("create hooks dir");
    let hook = hooks_dir.join("pre-commit");
    std::fs::write(&hook, "#!/usr/bin/env sh\necho 'custom hook'\n").expect("write hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
            .expect("chmod hook");
    }

    let output = doctor_in(&dir, &["--human"]).output().expect("run doctor");

    assert_eq!(output.status.code(), Some(1), "should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("✗ Hook installed"),
        "Hook installed should fail; got: {stdout}"
    );
    assert!(
        stdout.contains("⚠ Hook executable"),
        "Hook executable should be skipped; got: {stdout}"
    );
}

#[test]
fn test_r_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("r_sample.R"))
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
fn test_julia_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("julia_sample.jl"))
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
fn test_csharp_complexity() {
    let output = pretender_cmd()
        .arg("complexity")
        .arg(source_fixture("csharp_sample.cs"))
        .output()
        .expect("failed to execute process");

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
        stdout.contains("with-branch: 2"),
        "expected with-branch: 2 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("complex-func: 5"),
        "expected complex-func: 5 in stdout: {stdout}"
    );
}

/// Helper: set up a temp dir with a pretender.toml, a test file, and a JUnit XML report.
fn duration_setup() -> (PathBuf, PathBuf, PathBuf) {
    let (dir, _guard) = write_temp_file(
        "pretender.toml",
        r#"
        [thresholds.unit-test]
        duration_max_ms = 50
        [roles]
        test = { paths = [] }
        unit-test = { paths = [] }
        integration-test = { paths = [] }
        "#,
    );
    // Create the test file so role detection can find it
    let test_file = dir.join("tests/unit/test_math.py");
    std::fs::create_dir_all(test_file.parent().unwrap()).expect("create dirs");
    std::fs::write(&test_file, "def test_addition(): pass").expect("write test file");

    let junit_xml = format!(
        r#"<?xml version="1.0"?>
    <testsuite name="ts" tests="1">
        <testcase classname="test_math" name="test_addition" file="{}" time="0.1"/>
    </testsuite>"#,
        test_file.display()
    );
    let (_, report_path) = write_temp_file("test-report.xml", &junit_xml);
    (dir, test_file, report_path)
}

#[test]
fn test_duration_report_human_output() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("human")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .expect("pretender check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Test duration findings"),
        "expected 'Test duration findings' in stdout: {stdout}"
    );
    assert!(
        stdout.contains("test_addition"),
        "expected test name in stdout: {stdout}"
    );
    assert!(
        stdout.contains("100ms > 50ms"),
        "expected duration violation in stdout: {stdout}"
    );
}

#[test]
fn test_duration_report_json_contains_findings() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("json")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .expect("pretender check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test_findings"),
        "expected 'test_findings' in JSON output: {stdout}"
    );
    assert!(
        stdout.contains("test_addition"),
        "expected test name in JSON: {stdout}"
    );
}

#[test]
fn test_duration_report_sarif_contains_findings() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("sarif")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .expect("pretender check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("pretender/test-duration"),
        "expected 'pretender/test-duration' rule id in SARIF: {stdout}"
    );
    assert!(
        stdout.contains("test_addition"),
        "expected test name in SARIF: {stdout}"
    );
}

#[test]
fn test_duration_no_threshold_no_findings() {
    let (dir, _guard) = write_temp_file(
        "pretender.toml",
        r#"
        [roles]
        test = { paths = [] }
        unit-test = { paths = [] }
        integration-test = { paths = [] }
        "#,
    );
    let test_file = dir.join("tests/unit/test_math.py");
    std::fs::create_dir_all(test_file.parent().unwrap()).expect("create dirs");
    std::fs::write(&test_file, "def test_addition(): pass").expect("write test file");

    let junit_xml = format!(
        r#"<?xml version="1.0"?>
    <testsuite name="ts" tests="1">
        <testcase classname="test_math" name="test_addition" file="{}" time="0.1"/>
    </testsuite>"#,
        test_file.display()
    );
    let (_, report_path) = write_temp_file("test-report.xml", &junit_xml);

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("json")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .expect("pretender check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("test_findings"),
        "expected no test_findings when no threshold configured: {stdout}"
    );
}

#[test]
fn test_duration_cache_persisted() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("json")
        // raw build: tests exit policy (gate default) via its own config dir,
        // helper flags would shadow it (pretender-1te)
        .arg("--mode")
        .arg("tiered")
        .arg("--advisory-until")
        .arg("2099-12-31")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .expect("pretender check");
    assert!(output.status.success());

    // Check that the cache file contains test_findings
    let cache_path = dir.join(".pretender/last-check.json");
    assert!(cache_path.exists(), "cache file should exist");
    let cache_content = std::fs::read_to_string(&cache_path).expect("read cache");
    assert!(
        cache_content.contains("test_findings"),
        "expected test_findings in cache: {cache_content}"
    );
}

#[test]
fn test_doctor_warns_when_global_hooks_path_redirects_hook() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n").expect("write config");
    write_pretender_hook(&dir);

    // Global git config points core.hooksPath elsewhere (the lefthook-shim
    // scenario): git will invoke hooks from the redirected dir, so the
    // installed .git/hooks/pre-commit is inert.
    let global_cfg = dir.join("redirect.gitconfig");
    std::fs::write(
        &global_cfg,
        "[core]\n\thooksPath = /nonexistent-shim-hooks\n",
    )
    .expect("write redirect gitconfig");

    let mut cmd = pretender_cmd();
    cmd.arg("doctor")
        .arg("--human")
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .env("GIT_CONFIG_GLOBAL", &global_cfg);
    let output = cmd.output().expect("run doctor");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hook location"),
        "expected a 'Hook location' doctor check; got: {stdout}"
    );
    assert!(
        stdout.contains("core.hooksPath"),
        "warning should mention core.hooksPath; got: {stdout}"
    );
    assert!(
        stdout.contains("global"),
        "warning should name the config scope; got: {stdout}"
    );
}

#[test]
fn test_doctor_hook_location_passes_when_hooks_path_default() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n").expect("write config");
    write_pretender_hook(&dir);

    let output = doctor_in(&dir, &["--human"]).output().expect("run doctor");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The check runs but must not warn under the default (no redirect).
    assert!(
        stdout.contains("✓ Hook location"),
        "hook-location check should pass with default hooksPath; got: {stdout}"
    );
}
