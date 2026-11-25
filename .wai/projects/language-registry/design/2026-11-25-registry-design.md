# Design: Declarative Language Registry

Ticket: pretender-b5us.2 · Input: `research/2026-11-25-dispatch-surface.md` · 2026-11-25

## Decision

**Option A — data records + one generic parser** (research §5). Trait-per-language (B) rejected: it kills the match but keeps 13× boilerplate, a half-measure.

**What a new language touches, in one sentence:** add `languages/<lang>/plugin.toml` + `metrics.scm`, one `Language` enum variant, one `Grammar` enum variant (grammar binding), one registry record, and round-trip fixtures/tests — **no dispatch-site edits**.

**OCP, explicitly:** the registry is *open for extension* (extending = appending a `LangRecord` to the table) and *closed for modification* (no dispatch code changes — the extension match in `scan/parsers.rs`, the mirrored list in `collect/mod.rs`, and per-module boilerplate are all eliminated; the compiler's exhaustive `Grammar → tree_sitter::LANGUAGE` match is the single fixed point, and it only extends, never rewrites).

## Registry shape

```rust
// src/languages/registry.rs — data with data; cohesive.
pub(crate) struct LangRecord {
    pub grammar: Grammar,          // enum; fn LANGUAGE() -> tree_sitter::Language
    pub language: Language,        // model enum, consumed downstream
    pub extensions: &'static [&'static str],
    pub manifest: &'static str,    // include_str!("../<lang>/plugin.toml")
    pub query: &'static str,       // include_str!("../<lang>/metrics.scm")
}

pub(crate) static REGISTRY: OnceLock<Vec<LangRecord>> = ...; // 13 records (14 incl. TSX dual record)
```

- `GenericParser { record: &'static LangRecord, manifest: OnceLock<...> }` implements `model::Parser` once: `engine()` = `QueryEngine::new_with_branch_weights(record.grammar.language(), record.language, record.query, WeightMaps::from_manifest(m))`. Caching semantics preserved (OnceLock per record; static lifetime).
- TS/TSX: two records, same grammar + query, different `Language` variant — the current dual-parser quirk becomes pure data.
- `Grammar` enum + its `LANGUAGE()` match is the **one inherently non-data bit** (research constraint 3): 13 static grammar tokens, extended only when a new grammar is compiled in.
- WeightMaps today borrows `&m.branches` / `&m.smell_weights` from a cached manifest — unchanged: manifest parse stays OnceLock, registry reads through it.

## Ownership rules (which module owns which concern)

| concern | owner today | owner after |
|---|---|---|
| per-language data (query, manifest, extensions) | 13 `.rs` files duplicating it | `languages/<lang>/` files (unchanged location), read via registry |
| grammar binding | 13 `.rs` files | `Grammar` enum, one arm per grammar |
| extension dispatch | `scan/parsers.rs` two-arm match + `collect/mod.rs` mirror | registry lookup, single source |
| parser construction | 13 `impl Parser` | one `GenericParser`, `scan/parsers.rs::get_parser` becomes `REGISTRY.extension_lookup(ext)` |
| round-trip tests | inline `#[cfg(test)]` in 13 `.rs` modules + `tests/*_roundtrip_test.rs` | `tests/<lang>_roundtrip_test.rs` (integration; fixtures byte-pinned, unchanged) |
| mutation language subset | `mutation/mod.rs::detect_language` | unchanged enum, extension set derived from registry subset |
| plugin contract | `plugin.rs` (`#![allow(dead_code)]`) | load-bearing: registry parses its manifests; allow dropped |

## Migration path (every language green at every step)

Each step is one commit; `just ci` + golden-check (byte-identity) green per step.

1. **Add, don't move**: introduce `registry.rs` + `GenericParser`; `parser_for_extension*` delegates lookups to the registry while the 13 structs still exist. All tests pass untouched.
2. **Move tests**: inline round-trip tests from the 13 modules migrate to `tests/<lang>_roundtrip_test.rs` (one language per commit; pure moves, `git log --follow` clean).
3. **Delete boilerplate**: remove the 13 parser structs + their module files; `lib.rs` re-exports shrink to the registry. Fixtures byte-identical prove no behavior change.
4. **Single-source extensions**: `collect::is_supported_extension` derives from `REGISTRY`; mirrored const deleted.
5. **Wire mutation**: `detect_language` extension set derives from the registry subset (keep `MutationLang`).
6. **Drop the dead-code allow** in `plugin.rs`; `tooling_test::plugins_test` extended to pin every builtin manifest parses via the contract.

## Rollback story

No persisted state, no schema, no spec change, no findings delta — every step is an ordinary commit, so `git revert <step>` in reverse order is a complete rollback at any point; between steps the working tree is always a shippable, gate-green binary. The only external contract (SARIF/CLI/output) is untouched by construction, and `report_format_test` + golden-check run per step to prove it.

## Open questions

- None blocking. `shebangs` remains declaratively present in manifests (unused by builtin dispatch, extension-only, as today) until external plugins need it.
