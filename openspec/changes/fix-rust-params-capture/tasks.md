# Tasks: fix-rust-params-capture

- [ ] 1. Red test: `config.rs` test module gets `rust_sample_function_yields_nonzero_params` asserting `params.len() > 0` for a small inline Rust sample parsed by `RustParser` (no local named `raw`).
- [ ] 2. Green: extend `engine.rs` `extract_params` to read `pattern` field (fallback to `name`) and handle `self_parameter`/`self`; maps to spec scenario "rust parameters captured".
- [ ] 3. Refactor/pass-to-pass: existing engine round-trip tests stay green (`cargo test -p pretender rust`, one targeted round).
