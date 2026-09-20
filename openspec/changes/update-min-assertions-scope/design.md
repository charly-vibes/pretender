# update-min-assertions-scope — Design

## Choice: rule-level test identity vs role-level fixture handling

Two options were considered for exempting fixtures/helpers from `min_assertions`:

| Option | Mechanism | Rejected/Chosen |
|---|---|---|
| A. Fixture role via config | Add fixture path globs to `[roles]` config (e.g. exclude `tests/fixtures/**` from test role) | **Rejected** — changes role assignment for those files, which alters every rule's behavior on them and requires config edits in every adopting repo; also conflicts with the 68j anti-goal of not narrowing scanned scope via config |
| B. Rule-level test identity (chosen) | `min_assertions` evaluates only units that are themselves tests | **Chosen** — minimal blast radius: one rule, one evaluation check, no config change, no role change; complexity/params/etc. rules keep flagging oversized fixtures |

## Test identity definition

A unit has test identity when either:

1. **Attribute**: the unit's definition carries a test-registration attribute
   known for the language (Rust: `#[test]`; extensible per-language in the
   query). Verified by capturing `#[test]` on `function_item` in
   `languages/rust/metrics.scm`.
2. **Name pattern**: the unit name matches `^test_`, `_test$`, `^test[A-Z]`,
   `Test$`, or `^test$`. Language-agnostic; covers Python (`test_foo`),
   Rust (`test_adds`), Julia (`check` via `@testset` naming is out of scope —
   Julia units inside test files keep the attribute-free name path).

Out of scope for this change: Result-`?`-returning Rust tests counting as an
assertion form (parser-level concern, belongs to a 5bw follow-up), and any
new configuration keys.

## Risk

- Python `unittest` methods (`test_...` inside classes) already match the name
  pattern; no regression expected.
- A genuine test named e.g. `it_adds` would now be exempt — accepted MVP
  trade-off; the name pattern list is the extension point if this shows up.
