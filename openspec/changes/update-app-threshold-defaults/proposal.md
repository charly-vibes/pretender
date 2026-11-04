# Change: Re-evaluate app threshold defaults — tighten function length and enable shipped checks

## Why

Dogfooding pretender on its own Rust codebase (611 app units, `check --show-all`, v0.10.0) shows the headline
complexity limits are mainstream, not lax — but two things genuinely are:

1. **`function_lines_max = 40` is the lax outlier.** Dogfood p95 is 47 lines (already above the limit);
   reference points: Rubocop `MethodLength` defaults to 10, teams commonly run 20–30.
2. **Seven shipped checks default to disabled** (`unwrap_max`, `void_mutators_max`, `bool_cluster_max`,
   `mut_ratio_max`, `lazy_cluster_min`, `inheritance_depth_max`, `mock_count_max` — all `0`/off), which was a
   deliberate non-breaking choice at each check's introduction, but contradicts the values documented in
   `openspec/changes/CONVENTIONS.md` (`5 / 3 / 3 / 0.3 / 3 / 2 / 5`). "Lax" is mostly the OFF defaults.

Distribution evidence (app role, pretender/src, 611 units):

| metric | p50 | p90 | p95 | p99 | max | default | violations @default | @proposed |
|---|---|---|---|---|---|---|---|---|
| cyclomatic | 1 | 4 | 6 | 11 | 22 | 10 | 8 | keep 10 |
| cognitive | 0 | 4 | 9 | 23 | 63 | 15 | 12 | keep 15 |
| function_lines | 10 | 33 | 47 | 105 | 278 | 40 | 41 | 71 @30 |
| nesting | 0 | 2 | 3 | 4 | 6 | 3 | 15 | keep 3 |
| abc | 4 | 16 | 21 | 49 | 140 | 30 | 11 | keep 30 |
| params | — all 0 — | | | | | 4 | 0 | keep (unenforced for Rust — separate bug) |

Tightening cyclomatic 10→8 / cognitive 15→12 / nesting 3→2 adds only 2/3/23 findings here and departs from
mainstream defaults (McCabe/Sonar 10, Sonar cognitive 15, Rubocop nesting 3) — rejected for now; revisit with
more multi-language dogfood data.

## What Changes

- **Tighten** `[thresholds] function_lines_max` **40 → 30** (app role).
- **Enable previously-disabled shipped checks** at the values documented in `CONVENTIONS.md`:
  - `unwrap_max = 5` (global); `[thresholds.test] unwrap_max = 5` too — `EffectiveThresholds::for_role` reads
    unwrap from the test table with **no app fallback** (roles.rs `for_role`), so the test row is required, and it
    matches the shipped unwrap spec's stated test default (code had shipped 0)
  - `void_mutators_max = 3` (app role only; the test-role void check stays disabled at 0 — the void-mutator
    spec targets production code, and no test-role default is documented)
  - `[thresholds.test] lazy_cluster_min = 3` (min cluster size to flag; field lives in the test table,
    not the global one — config.rs `TestThresholds`)
  - `bool_cluster_max = 3`, `mut_ratio_max = 0.3`, `inheritance_depth_max = 2`
  - `[thresholds.test] mock_count_max = 5`
- **Add threshold-preset selection to `pretender init`** (opt-in strictness at setup time):
  - Interactive init asks `Threshold preset (default/strict) [default]:` after the mode question.
  - `pretender init --preset <default|strict>` for non-interactive runs (default `default`).
  - When init writes a fresh `pretender.toml` (init skips existing configs, main.rs "already exists, skipping"),
    it emits the chosen preset's threshold tables explicitly — the generated config is self-documenting and
    survives future default changes. Trade-off, intentional: a project initted today keeps today's values when
    a later version tightens the built-in defaults; re-init or hand-edit to pick up new defaults.
  - The `strict` preset (pinned in the spec delta): app `cyclomatic_max 8`, `cognitive_max 12`,
    `function_lines_max 25`, `nesting_max 2`, `params_max 3`, `abc_max 25`, `duplication_pct_max 4`,
    `file_lines_max 300`; test `cyclomatic_max 2`, `cognitive_max 3`, `function_lines_max 60`, `nesting_max 2`;
    enabled-check values identical to the default preset. Value rationale: dogfood p95/p99 headroom (see Why).
  - When `gate` mode + `strict` preset are combined, init prints the `check --adopt` / tiered-lease migration
    hint — strict on an existing codebase otherwise produces an immediate wall of violations.
- **Keep unchanged**: `cyclomatic_max 10`, `cognitive_max 15`, `nesting_max 3`, `params_max 4` (unenforced for Rust until the params extractor gap is fixed — see Dependencies), `abc_max 30`,
  `duplication_pct_max 5`, `file_lines_max 400`, `primitive_param_check false`, severity bands, script/library/test
  structural thresholds.
- **Pin the defaults in the spec**: modify the `Configuration Schema` requirement (`cli-and-config`) to add a
  scenario enumerating the default threshold table, so future drift is a spec violation, not just a test failure.
- Update `default_config_matches_documented_conventions` in `config.rs` to assert the new values (TDD: this test
  goes red first).
- Update documented defaults (README threshold examples, book, init generated config — now the preset tables —
  and CHANGELOG).
- **Migration for gate-mode users**: new findings appear after upgrade without config changes. Documented path:
  `pretender check --adopt` to baseline current state, or switch to `mode = "tiered"` with an advisory lease.
  New projects opt into strictness at `init` time via the preset question instead of inheriting a default they
  never see.
  Rule IDs, SARIF schema, and metric semantics are unchanged — defaults only.
- Out of scope (filed separately): Rust extractor does not populate `params` (so `params_max` is unenforced for
  Rust); coupling/cohesion defaults (check not yet implemented — governed by
  `add-coupling-and-cohesion-metrics`).

## Impact

- Affected specs: `cli-and-config` (MODIFIED Configuration Schema; ADDED Threshold Preset Selection, Strict
  Preset Values, Strict Preset Gate Hint).
- Affected code: `pretender/src/config.rs` (Default impls + conventions test), `pretender/src/main.rs`
  (`InitOptions` preset field, `prompt_init_options` question, `--preset` flag, `render_init_config` preset
  table emission, strict+gate adoption hint), docs (README, book, CHANGELOG).
- Breaking-ish for gate-mode users who never set explicit thresholds: previously-disabled checks become active.
  Mitigated by `--adopt` baseline and tiered mode; SARIF output compatibility untouched (no rule ID or schema
  changes).
- Known-new dogfood debt on pretender itself: ~71 function-length findings @30 (vs 41 @40); absorbed via
  `--adopt` baseline in this repo's lefthook config, cleaned up incrementally.
- Rule-of-5 review findings applied: test-table unwrap row added (no-fallback override in `for_role`),
  `lazy_cluster_min` moved to the test table, "unchanged" wording corrected, preset workstream added with
  pinned strict values and non-interactive/gate-migration coverage.

## Dependencies

- None blocking. Separate follow-up issue for the Rust `params` extraction gap.
