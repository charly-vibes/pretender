---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:review]
---

RO5U: reviewer flagged callee exact-match as HIGH (method calls won't match). Mitigated: eval/exec/compile are Python builtins always called bare — test confirms. Addressed HIGH test coverage gap: added exec and compile assertions with exact ABC values. Threading call_weights through 4 fns is MEDIUM design note — deferred to future CaptureMap refactor.
