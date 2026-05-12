---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-jy9-pure-metric-functions, pipeline-step:plan]
---

pretender-jy9 pure metric functions plan: Desired end state is metrics.rs exposes pure functions cyclomatic(&CodeUnit)->u32, cognitive(&CodeUnit)->u32, function_lines(&CodeUnit)->u32, params(&CodeUnit)->u32, nesting_max(&CodeUnit)->u32, and abc(&CodeUnit)->f64 over the universal model, with CyclomaticComplexity delegating to cyclomatic for existing CLI behavior. Out of scope: CLI presentation of all metrics, config thresholds, parser extraction of assignments/calls, and full future smell-weight configuration. Phases: RED add unit tests constructing CodeUnit/Block trees for branch recursion, logical sequence cognitive handling, line/param/nesting values, and ABC components; GREEN implement minimal recursive walkers; RO5U review spec alignment/edge cases; VERIFY run cargo test -p pretender.
