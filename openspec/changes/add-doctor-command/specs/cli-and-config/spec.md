## ADDED Requirements

### Requirement: Doctor Command

The system SHALL provide `pretender doctor` to run a sequential series of environment and configuration health checks and report a pass/fail verdict for each.

The checks SHALL be performed in this order:
1. **Git context** — the working directory is inside a git repository (detected via `git rev-parse --git-dir`)
2. **Config present** — `pretender.toml` exists in the working directory
3. **Config valid** — `pretender.toml` is valid TOML and passes schema validation; an empty file SHALL be treated as all-defaults and considered valid
4. **Hook installed** — a Pretender-managed pre-commit hook exists at `.git/hooks/pre-commit`; a hook is Pretender-managed if and only if it contains the string `# Installed by Pretender.` (the value of `PRE_COMMIT_HOOK_MARKER`)
5. **Hook executable** — the hook file has the executable bit set
6. **Plugin manifests** — any `.toml` files in the external metrics directory (`$PRETENDER_METRICS_DIR` or `~/.config/pretender/metrics/`) are well-formed TOML; if no metrics directory exists or it is empty, this check trivially passes

The explicit dependency edges are:
- **Config valid** and **Plugin manifests** depend on **Config present**
- **Hook installed** and **Hook executable** depend on **Git context**
- **Hook executable** depends on **Hook installed**

A check whose prerequisite failed SHALL be skipped and reported with status `skip` (`⚠`) rather than run. A skipped check SHALL NOT count as a failure for the purpose of the exit code.

The command SHALL exit with code `0` when all non-skipped checks pass, and with code `1` when one or more checks return status `fail`.

The default format SHALL be `human`. The command SHALL support `--format human|json`. Human format prints one line per check prefixed with `✓` (pass), `✗` (fail), or `⚠` (skip), followed by a summary line. JSON format emits a JSON array of objects with the fields `name`, `status` (`pass`, `fail`, or `skip` — corresponding to `✓`, `✗`, and `⚠` respectively), and `message`. The command does not support `--output`; output is always written to stdout.

The summary line SHALL read `X/Y checks passed` where Y is always 6 (the total number of checks), regardless of how many were skipped.

#### Scenario: All checks pass

- **WHEN** `pretender doctor` is run in a repository with a valid `pretender.toml` and a Pretender-managed hook installed
- **THEN** the command exits with code `0` and every check line is prefixed with `✓`

#### Scenario: Missing config skips dependent checks

- **WHEN** `pretender doctor` is run in a git repository without `pretender.toml`
- **THEN** the command exits with code `1`; **Config present** is prefixed with `✗`; **Config valid** and **Plugin manifests** are prefixed with `⚠`; **Hook installed** and **Hook executable** still run independently

#### Scenario: Not in a git repository skips hook checks

- **WHEN** `pretender doctor` is run outside any git repository
- **THEN** **Git context** is prefixed with `✗`; **Hook installed** and **Hook executable** are prefixed with `⚠`

#### Scenario: Hook not installed skips executable check

- **WHEN** `pretender doctor` is run in a git repository with no Pretender-managed hook at `.git/hooks/pre-commit`
- **THEN** **Hook installed** is prefixed with `✗` and **Hook executable** is prefixed with `⚠`

#### Scenario: Non-Pretender hook does not satisfy hook check

- **WHEN** a `.git/hooks/pre-commit` exists that was installed by another tool and does not contain `# Installed by Pretender.`
- **THEN** **Hook installed** is prefixed with `✗` and **Hook executable** is prefixed with `⚠`

#### Scenario: JSON format emits structured results on failure

- **WHEN** `pretender doctor --format json` is run in a repository without `pretender.toml`
- **THEN** stdout is a JSON array where each element has `name`, `status`, and `message` fields; the Config present element has `"status": "fail"`; the command exits with code `1`

#### Scenario: Summary line always uses total check count

- **WHEN** `pretender doctor` completes with some checks passing and some failing or skipped
- **THEN** the final output line reports passed checks out of 6, e.g. `4/6 checks passed`
