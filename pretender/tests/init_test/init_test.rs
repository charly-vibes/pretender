//! `init` subcommand tests: config writing and foreign-block neutrality.
//!
//! # Responsibilities
//! Pin init's non-interactive config generation (mode override, languages,
//! roles block), interactive hook + CI workflow installation, rejection of
//! the removed `--defaults` flag, and byte-identical preservation of
//! wai/dont-authored AGENTS.md blocks across init, check, and doctor runs.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;

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
    std::fs::write(&agents, FOREIGN_AGENTS).unwrap_or_else(|e| panic!("seed AGENTS.md: {e:?}"));
    let output = init_in(dir)
        .arg("--non-interactive")
        .output()
        .unwrap_or_else(|e| panic!("run init: {e:?}"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    agents
}

#[test]
fn test_init_defaults_flag_is_rejected() {
    let dir = tempdir();

    let output = init_in(&dir)
        .arg("--defaults")
        .output()
        .unwrap_or_else(|e| panic!("run init: {e:?}"));

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
        .unwrap_or_else(|e| panic!("run init: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = std::fs::read_to_string(dir.join("pretender.toml"))
        .unwrap_or_else(|e| panic!("config exists: {e:?}"));
    assert!(config.contains("mode = \"gate\""), "config: {config}");
    assert!(
        config.contains("languages = [\"auto\"]"),
        "config: {config}"
    );
    assert!(config.contains("[roles.test]"), "config: {config}");
}

#[test]
fn test_init_leaves_foreign_managed_blocks_untouched() {
    let dir = tempdir();
    let agents = seed_foreign_blocks_and_init(&dir);
    let after_init = std::fs::read(&agents).unwrap_or_else(|e| panic!("read AGENTS.md: {e:?}"));
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
        .unwrap_or_else(|e| panic!("write fixture: {e:?}"));

    let check_output = check_default(&dir)
        .arg("--json")
        .output()
        .unwrap_or_else(|e| panic!("run check: {e:?}"));
    let check_text = format!(
        "{}{}",
        String::from_utf8_lossy(&check_output.stdout),
        String::from_utf8_lossy(&check_output.stderr)
    );
    assert!(
        !check_text.contains("managed_block_drift"),
        "check must not emit a managed-block drift finding: {check_text}"
    );

    let doctor_output = doctor_in(&dir, &["--json"])
        .output()
        .unwrap_or_else(|e| panic!("run doctor: {e:?}"));
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

    let after_checks =
        std::fs::read(&agents).unwrap_or_else(|e| panic!("re-read AGENTS.md: {e:?}"));
    assert_eq!(
        after_checks,
        FOREIGN_AGENTS.as_bytes(),
        "check/doctor must not rewrite foreign managed blocks"
    );
}

/// Runs `pretender init` interactively, answering the mode/preset/languages/
/// hook/workflow prompts, and asserts success.
fn init_interactive(dir: &std::path::Path) -> std::process::Output {
    let output = init_in(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write as _;
            child
                .stdin
                .as_mut()
                .unwrap_or_else(|| panic!("stdin"))
                // mode, preset (added by update-app-threshold-defaults),
                // languages, hook, workflow
                .write_all(b"guidance\n\npython,rust\ny\ny\n")?;
            child.wait_with_output()
        })
        .unwrap_or_else(|e| panic!("run init interactively: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn test_init_interactive_can_install_hook_and_ci() {
    let dir = tempdir();
    git_init(&dir);

    init_interactive(&dir);

    let config = std::fs::read_to_string(dir.join("pretender.toml"))
        .unwrap_or_else(|e| panic!("config exists: {e:?}"));
    assert!(config.contains("mode = \"guidance\""), "config: {config}");
    assert!(
        config.contains("languages = [\"python\", \"rust\"]"),
        "config: {config}"
    );

    let hook = std::fs::read_to_string(dir.join(".git/hooks/pre-commit"))
        .unwrap_or_else(|e| panic!("hook exists: {e:?}"));
    assert!(hook.contains("Installed by Pretender"), "hook: {hook}");
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
    );

    let workflow = std::fs::read_to_string(dir.join(".github/workflows/pretender.yml"))
        .unwrap_or_else(|e| panic!("workflow exists: {e:?}"));
    assert!(
        workflow.contains(
            "cargo install --git https://github.com/charly-vibes/pretender --locked pretender"
        ),
        "workflow: {workflow}"
    );
}
