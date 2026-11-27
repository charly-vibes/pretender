---
tags: [pipeline-run:tdd-ro5-2026-11-27-pretender-jg9y-four-heuristic-gaps, pipeline-step:orient]
---

ORIENT: pretender-jg9y — four heuristic gaps.
1. Rust min_assertions counts 0 for assert_cmd method chains; fix = method-call assertion captures in languages/rust/metrics.scm, each chain link counted.
2. unwraps: unwrap_or* already not matched by ".unwrap(" substring scan (pin with fixture); exclude .expect( sites inside #[cfg(test)]-gated modules when file role is test (new detect_unwraps_with_role).
3. mock_count fires on tests/support/* helpers because rust usage pattern ".expect(" matches error plumbing; fix = skip mock detection for boundary-aligned `support` path segment (roles.rs is_test_support_path), keeping leaf *_test.rs firing.
4. exported_params silent in full-repo gate: walked paths carry "./" prefix, role globs like "sad/src/**" don't match, role falls back to app whose ExportRules::none(); fix = glob_role also tries walker-relative path (mirrors exclude.rs strip_leading_cur_dir).

