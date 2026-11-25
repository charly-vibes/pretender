//! Tests for the init command's preset handling.
//!
//! # Responsibilities
//! Contract coverage for preset threshold tables, preset-prompt
//! suppression, rejection of unknown presets, and the gate+strict
//! migration hint.
//!
//! # Rationale
//! Split from init/mod.rs to stay within the interim file-size limit; the
//! oversized scenario test was decomposed into per-scenario helpers that
//! preserve assertion order, and panicking helpers keep the file
//! unwrap-free.
use super::*;
use pretender::config::Config;

/// Run the CLI binary in `dir` with a neutral GIT_CONFIG_GLOBAL so
/// hooksPath resolution never sees the developer machine's real global
/// config (same hermeticity pattern as tests/cli_test.rs pretender_cmd()).
fn init_command(dir: &Path, args: &[&str]) -> std::process::Command {
    let bin =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/pretender");
    let neutral = std::env::temp_dir().join(format!(
        "pretender-init-preset-test-gitconfig-{}",
        std::process::id()
    ));
    std::fs::write(&neutral, "").unwrap_or_else(|e| panic!("write neutral gitconfig: {e}"));
    let mut cmd = std::process::Command::new(bin);
    cmd.current_dir(dir).env("GIT_CONFIG_GLOBAL", neutral);
    cmd.args(args);
    cmd
}

/// Run `init` to completion with the hermetic environment.
fn init_run(dir: &Path, args: &[&str]) -> std::process::Output {
    init_command(dir, args)
        .output()
        .unwrap_or_else(|e| panic!("run init {args:?}: {e}"))
}

fn expect_success(out: &std::process::Output, context: &str) {
    assert!(
        out.status.success(),
        "{context}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn read_config(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("pretender.toml"))
        .unwrap_or_else(|e| panic!("config written: {e}"))
}

/// Contract test
/// (.espectacular/cli-and-config/interactive-init-asks-for-preset.toml):
/// the preset choice — prompt answer or --preset flag — selects the
/// threshold tables init emits; an explicit --preset suppresses the
/// interactive preset prompt (mirroring --mode).
#[test]
fn init_preset_prompt_writes_threshold_tables() {
    strict_preset_tables_written();
    default_preset_tables_written();
    absent_flag_writes_default_tables();
    explicit_preset_suppresses_preset_prompt();
}

/// --preset strict --non-interactive: strict tables, app cyclomatic 8.
/// Task 3.3: emitted strict TOML parses back to the pinned strict
/// effective values (spec: Strict Preset Values) via toml::from_str.
fn strict_preset_tables_written() {
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let out = init_run(
        dir.path(),
        &["init", "--non-interactive", "--preset", "strict"],
    );
    expect_success(&out, "init --preset strict");
    let strict_config = read_config(dir.path());
    assert!(
        strict_config.contains("cyclomatic_max = 8"),
        "strict app table missing: {strict_config}"
    );
    let parsed: Config =
        toml::from_str(&strict_config).unwrap_or_else(|e| panic!("strict config parses: {e}"));
    assert_eq!(parsed.thresholds.app.cyclomatic_max, 8);
    assert_eq!(parsed.thresholds.app.cognitive_max, 12);
    assert_eq!(parsed.thresholds.app.function_lines_max, 25);
    assert_eq!(parsed.thresholds.app.nesting_max, 2);
    assert_eq!(parsed.thresholds.app.params_max, 3);
    assert_eq!(parsed.thresholds.app.abc_max, 25);
    assert_eq!(parsed.thresholds.app.duplication_pct_max, 4);
    assert_eq!(parsed.thresholds.app.file_lines_max, 300);
    assert_eq!(parsed.thresholds.test.cyclomatic_max, 2);
    assert_eq!(parsed.thresholds.test.cognitive_max, 3);
    assert_eq!(parsed.thresholds.test.function_lines_max, 60);
    assert_eq!(parsed.thresholds.test.nesting_max, 2);
    // enabled-check values identical to the default preset
    assert_eq!(parsed.thresholds.app.unwrap_max, 5);
    assert_eq!(parsed.thresholds.test.mock_count_max, 5);
}

/// --preset default writes the default tables, which must round-trip to
/// the built-in defaults.
fn default_preset_tables_written() {
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let out = init_run(
        dir.path(),
        &["init", "--non-interactive", "--preset", "default"],
    );
    expect_success(&out, "init --preset default");
    let default_config = read_config(dir.path());
    assert!(
        default_config.contains("cyclomatic_max = 10"),
        "default app table missing: {default_config}"
    );
    let parsed: Config =
        toml::from_str(&default_config).unwrap_or_else(|e| panic!("default config parses: {e}"));
    assert_eq!(
        parsed.thresholds.app,
        Config::default().thresholds.app,
        "default preset tables must round-trip to the built-in app defaults"
    );
    assert_eq!(
        parsed.thresholds.test,
        Config::default().thresholds.test,
        "default preset tables must round-trip to the built-in test defaults"
    );
}

/// Absent flag: same as --preset default.
fn absent_flag_writes_default_tables() {
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let out = init_run(dir.path(), &["init", "--non-interactive"]);
    expect_success(&out, "init without --preset");
    assert!(
        read_config(dir.path()).contains("cyclomatic_max = 10"),
        "absent --preset must emit the default tables"
    );
}

/// Explicit --preset on an interactive run suppresses the preset
/// prompt: stdin answers mode/languages/hook/workflow only; had the
/// prompt not been suppressed, the answers would shift and the run
/// would fail (or fall back to default tables).
fn explicit_preset_suppresses_preset_prompt() {
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let mut child = init_command(dir.path(), &["init", "--preset", "strict"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn interactive init: {e}"));
    {
        use std::io::Write as _;
        child
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("piped stdin"))
            .write_all(b"\nauto\nn\nn\n")
            .unwrap_or_else(|e| panic!("write prompt answers: {e}"));
    }
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("interactive init finishes: {e}"));
    expect_success(&out, "interactive init with explicit --preset");
    assert!(
        read_config(dir.path()).contains("cyclomatic_max = 8"),
        "explicit --preset must suppress the preset prompt and emit strict tables"
    );
}

#[test]
fn init_rejects_unknown_preset() {
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let out = init_run(
        dir.path(),
        &["init", "--non-interactive", "--preset", "loose"],
    );
    assert!(!out.status.success(), "unknown preset must exit non-zero");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("loose"),
        "error should name the bad value: {err}"
    );
}

#[test]
fn init_strict_gate_prints_migration_hint() {
    // Spec: Strict Preset Gate Hint — gate + strict must print the
    // --adopt / tiered-lease migration hint after writing the config.
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let out = init_run(
        dir.path(),
        &[
            "init",
            "--non-interactive",
            "--mode",
            "gate",
            "--preset",
            "strict",
        ],
    );
    expect_success(&out, "init gate+strict");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("check --adopt"),
        "gate+strict must print the adoption hint; stdout: {stdout}"
    );
}

#[test]
fn init_default_gate_no_migration_hint() {
    // The hint is scoped to gate+strict; default preset stays quiet.
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let out = init_run(dir.path(), &["init", "--non-interactive", "--mode", "gate"]);
    expect_success(&out, "init gate+default");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("check --adopt"),
        "gate+default must not print the adoption hint; stdout: {stdout}"
    );
}
