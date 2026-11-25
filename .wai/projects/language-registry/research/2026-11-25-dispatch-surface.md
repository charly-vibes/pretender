# Research: Language-Dispatch Surface and Plugin Contract

Ticket: pretender-b5us.1 · Branch: `research/language-registry` · 2026-11-25 · HEAD `224b744` (post 5zo.15 merge)

## 1. Dispatch map (file:line)

The epic description cites `engine.rs::parse` (cyc 26 / cog 89) — **stale**: the R3 ratchet container deleted `engine.rs`; `QueryEngine` is generic (no per-language branching). The branch tree now lives in the scan layer:

| site | location | shape | growth cost |
|---|---|---|---|
| **extension → parser** | `pretender/src/check/scan/parsers.rs:22-46` | two `match ext` arms (`parser_for_extension`, `parser_for_extension_rest` — split only to satisfy the function-size limit) | +1 arm (×2 functions) + import + lib export |
| **supported-extension set** | `pretender/src/check/collect/mod.rs:90-97` | flat `const EXTENSIONS: &[&str]` — textual mirror of parsers.rs (29 extensions); drift guarded by a doc comment only | +1 string |
| **mutation languages** | `pretender/src/mutation/mod.rs:53-90` | `detect_language(path) -> Option<MutationLang>` — second, smaller extension dispatch (mutation-testable languages only) | independent |
| **per-language modules** | `pretender/src/{c,clojure,cpp,csharp,go,java,javascript,julia,python,r,ruby,rust,typescript}.rs` (13 files, 43–84 ln) | identical impl shape per language (see §2) | +1 file, boilerplate |
| engine | `pretender/src/engine/mod.rs` | **generic** — `QueryEngine::new_with_branch_weights(grammar, Language, query, WeightMaps)`; zero language branches | none |
| roles | `pretender/src/roles.rs` | path-pattern roles only; no language branches | none |

## 2. Duplication inventory (13× repeated shape)

Every language module repeats, differing only in data:

```rust
const X_QUERY: &str = include_str!("../languages/x/metrics.scm");     // data
const X_MANIFEST: &str = include_str!("../languages/x/plugin.toml");  // data
fn manifest() -> Result<&'static PluginManifest> { /* OnceLock boilerplate, ~10 ln */ }
impl XParser {
    fn engine(&self) -> Result<QueryEngine> {                          // shape-identical
        QueryEngine::new_with_branch_weights(
            tree_sitter_x::LANGUAGE.into(),                            // per-language: grammar binding
            Language::X,                                               // per-language: enum variant
            X_QUERY, &WeightMaps { branch: &m.branches, call: &m.smell_weights });
    }
}
impl Parser for XParser { fn parse(...) -> ... { self.engine()?.parse(...) } }  // pure delegation
```

- `manifest()` OnceLock boilerplate: ~10 ln × 13 — **accidental**.
- `engine()`: differs only in grammar token + enum variant + const names — **accidental given a registry**.
- `impl Parser` delegation: 4 ln × 13 — **accidental**.
- Extension data duplicated twice in code (parsers.rs arms + collect list) although `plugin.toml.extensions` already carries it per language — **accidental**; the manifest is parsed and cached today but its `extensions` field is never read by dispatch.
- `typescript.rs`: two parser structs (TS + TSX) sharing one manifest/grammar — same-shape duplicate.
- Accidental-vs-essential: per-language *data* (grammar token, enum variant, scm, toml) is essential; 100% of the Rust plumbing around it is boilerplate.

## 3. Plugin contract — required surface, written down

`pretender/src/plugin.rs` (108 ln, `#![allow(dead_code)]` — contract parsed+tested but **not wired to builtin dispatch**):

- `PluginManifest` (TOML, `from_toml`): `name`, `display_name`, `extensions: Vec<String>`, `shebangs: Vec<String>`, `tree_sitter {source, rev}`, `query: String`, `branches: BTreeMap<capture, {cyclomatic, cognitive}>` (default 1/1), `assertions.patterns: Vec<String>`, `smell_weights: BTreeMap<name, f64>`.
- 13 builtin `plugin.toml` files already conform (test-pinned for python; others parsed ad hoc inside each module's own `manifest()`).
- `external_plugin/{loader,parse}.rs`: the *external* plugin path (manifest-driven invoke, findings parse) — external plugins are already data-driven; **builtin languages are the ones that bypass the registry**.
- Gap the registry must close: `extensions` (dispatch), `tree_sitter` (grammar binding), and unit construction are data in TOML but code in the 13 modules.

## 4. Constraints a registry must satisfy

1. **No rule outcome change** — metrics per fixture byte-identical; round-trip fixtures green unchanged.
2. **No CLI change, no SARIF change** — rule IDs, report shapes, exit codes untouched.
3. **Pinned grammars**: `tree_sitter.source = "builtin:tree-sitter-*"` must resolve to statically compiled grammars; the one inherently non-data bit is grammar-token lookup (`grammar_id → tree_sitter::*::LANGUAGE`), acceptable as a small fixed table.
4. **`model::Language` enum stays** — metrics/report/match sites consume it; adding a language remains a one-line enum touch (exhaustive-match guard is a feature, not a violation).
5. **Caching semantics**: per-language manifest/engine must keep OnceLock-style single-init (engine construction is expensive).
6. **Mutation dispatch**: `detect_language` can delegate to the registry (subset filter), or stay as-is — must not regress `tooling_test` mutation coverage.
7. **Dead-code allow can drop** once `plugin.rs` is load-bearing (checks the wiring actually happened).
8. **ledger/gate neutral**: refactor only — zero findings delta; `just ci` green including golden-check byte-identity.

## 5. Registry shape options (input to the design child, b5us.2+)

- **A. Data records + one generic parser**: a `const REGISTRY: &[LangRecord]` (or OnceLock-built from the 13 manifests) mapping extension → {grammar token, Language variant, query, manifest}; one `GenericParser` implements `Parser` for all. Per-language `.rs` files dissolve into data + their unit tests. Est. −700 ln of boilerplate; new language = plugin.toml + metrics.scm + grammar table row + tests.
- **B. Trait-based adapter registry**: keep per-language modules, register `dyn Parser` factories in a HashMap. Kills the match but keeps 13× boilerplate — rejected as half-measure.
- Open for the design ticket: how TS/TSX dual parsers, `MutationLang` subset, and the `Language` enum variant binding express as data.

## 6. Verification hooks the refactor must keep green

`tests/python_roundtrip_test.rs` + 13 adapter round-trip unit tests (fixtures byte-pinned), `check_test` (57), `tooling_test` (51), `report_format_test` (byte-compat SARIF/JSON/human/markdown), golden-check, `just ci`.
