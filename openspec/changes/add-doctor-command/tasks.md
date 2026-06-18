## 1. Command Scaffold
- [ ] 1.1 Add `Doctor` variant to the `Commands` enum in `main.rs`
- [ ] 1.2 Create `pretender/src/doctor.rs` with a `run_doctor` function
- [ ] 1.3 Wire `Commands::Doctor` in `main.rs` to call `run_doctor`

## 2. Health Checks
- [ ] 2.1 Implement **Git context** check via `git rev-parse --git-dir`
- [ ] 2.2 Implement **Config present** check (`pretender.toml` exists in cwd)
- [ ] 2.3 Implement **Config valid** check (parse `pretender.toml` with existing `Config::load`; treat empty file as valid)
- [ ] 2.4 Implement **Hook installed** check (`.git/hooks/pre-commit` exists and contains `PRE_COMMIT_HOOK_MARKER`)
- [ ] 2.5 Implement **Hook executable** check (file permissions include executable bit)
- [ ] 2.6 Implement **Plugin manifests** check (scan `$PRETENDER_METRICS_DIR` or `~/.config/pretender/metrics/` for `.toml` files that fail to parse; pass trivially if directory absent or empty)
- [ ] 2.7 Enforce dependency edges: skip Config valid + Plugin manifests when Config present fails; skip Hook installed + Hook executable when Git context fails; skip Hook executable when Hook installed fails

## 3. Output
- [ ] 3.1 Human formatter: one line per check with `✓` / `✗` / `⚠` prefix and short message
- [ ] 3.2 JSON formatter: array of `{ name, status, message }` objects where status is `pass`, `fail`, or `skip`
- [ ] 3.3 Print summary line: `X/6 checks passed` (denominator always 6)
- [ ] 3.4 Exit code `0` when all non-skipped checks pass, `1` when any check fails; skipped checks do not affect exit code

## 4. Tests
- [ ] 4.1 Integration test: `pretender doctor` in a repo with valid config and hook exits `0`
- [ ] 4.2 Integration test: `pretender doctor` with missing `pretender.toml` exits `1`, names Config present as failing, and shows Config valid + Plugin manifests as skipped
- [ ] 4.3 Integration test: `pretender doctor --format json` with missing config emits valid JSON array with `"status": "fail"` for Config present and exits `1`
- [ ] 4.4 Integration test: hook exists but is not Pretender-managed → Hook installed is `✗`, Hook executable is `⚠`
