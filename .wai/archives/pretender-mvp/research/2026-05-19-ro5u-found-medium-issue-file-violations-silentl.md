---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:review]
---

RO5U: Found MEDIUM issue — file_violations silently omitted from SARIF (unit violations only were emitted). Fixed by refactoring push_result as a closure and adding file violation loop with startLine=1 (SARIF convention for file-scope findings). All 16 tests pass. Findings #2 (abc precision) and #3 (JSON schema change) accepted as low severity for MVP.
