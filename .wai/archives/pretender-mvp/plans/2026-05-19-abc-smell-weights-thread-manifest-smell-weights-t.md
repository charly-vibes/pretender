---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:plan]
---

ABC smell weights: thread manifest.smell_weights through QueryEngine to weight calls by callee name. Python: eval=5, exec=5, compile=3. metrics.rs already uses smell_weight; just need to set it from plugin. Exact match only for MVP. Out of scope: regex callee matching, non-Python weights.
