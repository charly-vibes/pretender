## 1. Configuration

- [ ] 1.1 Add `bool_cluster_max: u32` and `primitive_param_check: bool` to `[thresholds]` in `config.rs`; defaults `0` and `false`. Add `[patterns.primitive_params] extra: Vec<String>` for user-defined domain-name patterns (default empty). Validate: `bool_cluster_max` non-negative u32, `primitive_param_check` boolean.
- [ ] 1.2 Unit tests for config parse + defaults + validation + custom patterns.

## 2. Parameter type detection

- [ ] 2.1 Extend the existing `CodeUnit.parameters` to include an optional `type_name: Option<String>` field. This is populated from `@param.type` captures in language adapters.
- [ ] 2.2 For each language adapter, add `@param.type` capture to the function-parameter query pattern where type annotations exist (Rust, Java, TypeScript, C++, Go, C#, Julia). For Python, Ruby, JavaScript — types are optional; if absent for all params in a file, skip the check entirely for that file and emit a note in diagnostic output.
- [ ] 2.3 Implement boolean detection: `type_name` matches `bool`, `boolean`, `Boolean`, `bool?`, `Bool`.
- [ ] 2.4 Implement primitive domain parameter detection: `type_name` matches `String`, `str`, `int`, `i32`, `i64`, `u32`, `u64`, `Int`, `Integer`, `number`, `string` and param `name` matches regex `(email|url|uri|date|timestamp|phone|address|zip|ssn|uuid)`. Exclude common false-positive names: `id`, `name`, `path` — these are legitimate primitive uses (DB IDs, display names, file paths).
- [ ] 2.5 Exclude struct/class/record definitions from boolean cluster check — only flag bare `bool` params in function/method/constructor parameter lists. Configuration flag objects with multiple bool fields are legitimate.
- [ ] 2.6 Exclude files with role `generated` or `vendor` from all primitive-obsession analysis.
- [ ] 2.7 Unit tests: function with 4 bool params flagged; domain param with `email: String` flagged; function with 2 bool params not flagged at threshold 3; struct definition with 3 bool fields NOT flagged; dynamically typed file (no type annotations) produces no finding; generated/vendor file produces no finding.

## 3. Evaluation and reporting

- [ ] 3.1 Per code unit: count bool params, compare to `bool_cluster_max`. If `primitive_param_check` is true, scan for domain params.
- [ ] 3.2 Add `bool_cluster` and `primitive_param` violation types to `UnitReport.violations`.
- [ ] 3.3 Extend `decide_exit_code` for gate mode.
- [ ] 3.4 Human, JSON, SARIF output formats.

## 4. Validation

- [ ] 4.1 Run `openspec validate add-primitive-obsession-check --strict`.
- [ ] 4.2 Run `just ci` green.