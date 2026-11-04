# Tasks: update-app-threshold-defaults

## 1. Spec delta

- [ ] 1.1 Modify `Configuration Schema` requirement in `openspec/changes/update-app-threshold-defaults/specs/cli-and-config/spec.md`: add scenario enumerating default thresholds.
- [ ] 1.2 The two preset/default contract TOMLs live in `contracts/` inside this change (they reference scenarios that only exist in the overlay). On `openspec archive`, move them to `.espectacular/cli-and-config/`.

## 2. TDD: defaults

- [ ] 2.1 RED: update `default_config_matches_documented_conventions` in `pretender/src/config.rs` to assert new defaults (`function_lines_max = 30`, `unwrap_max = 5`, `void_mutators_max = 3`, `bool_cluster_max = 3`, `mut_ratio_max = 0.3`, `inheritance_depth_max = 2`, test `mock_count_max = 5`, test `unwrap_max = 5`, test `lazy_cluster_min = 3`). Run `just test` — must fail.
- [ ] 2.2 GREEN: update `Default` impls (`AppThresholds`, `TestThresholds`) in `config.rs` — note test `unwrap_max` is required because `EffectiveThresholds::for_role` reads it from the test table with no app fallback. `just test` green.
- [ ] 2.3 Check for tests elsewhere asserting old defaults (grep `40`, `mock_count_max = 0`, etc. in `pretender/tests/`); update expectations that pin old defaults, keep behavioral tests explicit-threshold where they don't intend to pin defaults.

## 3. TDD: init threshold presets

- [ ] 3.1 RED: test `init --preset strict --non-interactive` writes a `pretender.toml` containing the strict tables (app `cyclomatic_max = 8` etc.); `--preset default` (and absent flag) writes the default tables; unknown preset value rejected; explicit `--preset` on an interactive run suppresses the prompt. Name the test `init_preset_prompt_writes_threshold_tables` (matches `.espectacular/cli-and-config/interactive-init-asks-for-preset.toml`). Existing-config skip behavior unchanged (main.rs "already exists, skipping"). Run `just test` — must fail.
- [ ] 3.2 GREEN: add `preset` to `InitOptions` + `--preset <default|strict>` flag (main.rs:167,431), preset override mirroring `mode_override` (explicit flag suppresses the prompt), prompt in `prompt_init_options` after mode ("Threshold preset (default/strict) [default]: "), strict+gate adoption hint print (always for gate+strict), and `render_init_config` emits explicit threshold tables for the chosen preset. `just test` green.
- [ ] 3.3 Strict values live in one place (a `preset_tables(preset) -> String`-style helper in main.rs or config.rs) so the spec table and emitted TOML cannot drift; test asserts emitted strict TOML parses back to strict effective values via `Config::parse_str`.

## 4. Docs and generated config

- [ ] 4.1 Update README threshold table, book docs, and init/README text describing the preset question and `--preset` flag.
- [ ] 4.2 CHANGELOG entry under unreleased, flagging gate-mode migration (`--adopt` / tiered lease) and the new preset question/flag.

## 5. Dogfood and gates

- [ ] 5.1 Rebuild and run `pretender check --adopt` on this repo; commit refreshed `pretender-baseline.json` so lefthook stays green.
- [ ] 5.2 `ah check` + `openspec validate update-app-threshold-defaults --strict` before commit.
