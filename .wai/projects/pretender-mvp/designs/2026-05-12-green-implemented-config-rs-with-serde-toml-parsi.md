---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-sbq-config-parse-pretender-toml, pipeline-step:green]
---

GREEN: Implemented config.rs with serde+toml parsing for the documented schema, default sections/thresholds/bands/plugins/output/roles, unknown-key tolerance via serde defaults/no deny_unknown_fields, load_from_str/load_from_path helpers, and miette Diagnostic config errors for read/parse/validation failures. Validation currently enforces band ordering green <= yellow <= red. Added miette, thiserror, and toml dependencies. Verified config::tests and cargo test -p pretender --quiet green.
