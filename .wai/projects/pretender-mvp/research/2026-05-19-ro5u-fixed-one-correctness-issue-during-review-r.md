---
tags: [pipeline-run:tdd-ro5u-2026-05-19-pretender-oyg-min-assertions, pipeline-step:review]
---

RO5U: fixed one correctness issue during review: recursive assertion counting originally included captures inside nested helper functions, which would let outer tests satisfy min_assertions accidentally. count_captured_nodes now skips nested definitions, and a regression test covers parent vs nested helper counts.
