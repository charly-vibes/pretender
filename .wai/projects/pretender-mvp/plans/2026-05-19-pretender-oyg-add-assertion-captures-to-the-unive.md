---
tags: [pipeline-run:tdd-ro5u-2026-05-19-pretender-oyg-min-assertions, pipeline-step:plan]
---

pretender-oyg: add assertion captures to the universal model and query engine, count assertions per CodeUnit, and emit min_assertions violations only for role=test using plugin-defined @assert.* patterns. Desired end state: python test functions without assertions fail check; functions with assertions pass; non-test roles unchanged. Out of scope: new language assertion patterns beyond existing manifests, SARIF quick-fixes, and suppression syntax.
