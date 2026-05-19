---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:review]
---

RO5U: 2 High findings fixed. (1) @branch.switch_case was silently ignored by engine (not in branch_mapping hardcoded list) — renamed to @branch.if in both .scm files; switch cases now counted correctly as cyclomatic +1. (2) .tsx files were using LANGUAGE_TYPESCRIPT which fails on JSX syntax — added TypeScriptXParser using LANGUAGE_TSX, dispatched tsx/cts to it. Low findings noted: JS class methods always UnitKind::Function (pre-existing engine limitation), arrow expression-body edge case (deferred).
