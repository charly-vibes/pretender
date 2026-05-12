---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-sbq-config-parse-pretender-toml, pipeline-step:plan]
---

pretender-sbq config parse pretender.toml plan: Desired end state is a config module that parses pretender.toml with serde+toml into the OpenSpec schema sections ([pretender], [thresholds] role overrides, [bands], [scope], [execute], [plugins], [output], [roles]), supplies convention-aligned defaults, ignores unknown keys, validates loaded config with miette diagnostics, and exposes load_from_str/load_from_path helpers. Out of scope: wiring config into check/init CLI, role glob matching, executing coverage/mutation commands, and SARIF output. Phases: RED add parser/default/validation tests; GREEN implement config data types, defaults, toml parsing, and validation diagnostics; RO5U review spec alignment and edge cases; VERIFY cargo test -p pretender.
