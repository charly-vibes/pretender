# Implementation Status

Current build status for every command and feature. Stubbed items exit with
code `2` and a "not yet implemented" message. Partial items are functional
but have noted gaps.

---

## Commands

| Command | Status | Notes |
|---------|--------|-------|
| `pretender init` | ✅ Implemented | Interactive and `--defaults` modes |
| `pretender check` | ✅ Implemented | human, json, sarif output |
| `pretender complexity` | ✅ Implemented | Single-file, worst-first sort |
| `pretender report` | ✅ Implemented | human, markdown, html |
| `pretender duplication` | ✅ Implemented | Cross-file and single-file |
| `pretender mutation` | ✅ Implemented | Python, Rust, JS, TS — see caveats |
| `pretender hooks install` | ✅ Implemented | Writes pre-commit shim |
| `pretender hooks uninstall` | ✅ Implemented | Removes Pretender-managed shim |
| `pretender ci generate github` | ✅ Implemented | Writes `.github/workflows/pretender.yml` |
| `pretender explain` | ✅ Implemented | All built-in metrics |
| `pretender plugins` | ❌ Stub | Tracked: pretender-07m |

---

## Partial features

| Feature | Status | Detail |
|---------|--------|--------|
| `check --format junit` | ❌ Stub | Tracked: pretender-t2m |
| `check --format markdown` | ❌ Stub | Tracked: pretender-t2m |
| `ci generate <non-github>` | ❌ Stub | Only `github` is implemented |
| Mutation testing — Java | ⚠️ Partial | `--dry-run` works via tree-sitter; PIT is not invoked automatically |
| `check --staged` | ⚠️ Partial | Pre-commit shim runs `pretender check .`; true staged-file filtering is deferred |

---

## Language support matrix

| Language | Complexity | Mutation |
|----------|-----------|---------|
| Python | ✅ | ✅ (mutmut) |
| Rust | ✅ | ✅ (cargo-mutants) |
| JavaScript | ✅ | ✅ (Stryker) |
| TypeScript | ✅ | ✅ (Stryker) |
| Go | ✅ | — |
| Java | ✅ | ⚠️ dry-run only |
| Ruby | ✅ | — |
| C | ✅ | — |
| C++ | ✅ | — |
