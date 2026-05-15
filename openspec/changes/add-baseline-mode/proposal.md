# Change: Add Baseline Mode

## Why

Enabling `gate` mode on a legacy codebase requires all existing violations to be fixed first,
which is often impractical. Baseline mode snapshots accepted findings so that `gate` mode can be
turned on immediately: pre-existing violations are grandfathered in, while new or worsening
violations are blocked. The ratchet mechanism ensures the baseline can only shrink over time.

## What Changes

- New `baseline` capability: snapshot file `pretender.baseline.json` with fingerprinted findings
- `pretender baseline create` — captures current violations into the baseline snapshot
- `pretender baseline update` — re-snapshots after manual cleanup
- `pretender baseline show` — lists all grandfathered findings
- `pretender check --baseline` — fails only on findings absent from the baseline or worse than their baselined value
- Ratchet: if a baselined finding improves, the baseline entry is tightened without stderr output or exit-code change (logged at INFO as `baseline.tightened`); if it regresses, the run fails
- New `[baseline]` config table in `pretender.toml`

## Impact

- Affected specs: `baseline` (new), `cli-and-config` (new subcommands + config + flag on `check`)
- Affected code: `src/baseline/` (new), `src/cli/`, `src/check_runner.rs`
- Cross-cutting: fingerprint design is a significant decision; see `design.md`
- Fingerprints include the unit start line to disambiguate repeated or nested units with the same name; moving or renaming a unit intentionally creates a new baseline identity
