---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:plan]
---

pretender-t2m SARIF 2.1.0 output plan

End state: pretender check --format sarif emits valid SARIF 2.1.0 JSON.
- runs[0].tool.driver with rules array (one per distinct violated metric)
- runs[0].results for every unit-level violation: ruleId, level=warning, message.text (actual/limit), physicalLocation (file URI + startLine)
- All existing tests pass; new SARIF integration test passes

Changes needed:
1. Add start_line: u32 to UnitReport, populate from unit.span.start_line
2. Implement write_sarif_report() using serde-sarif builder API
3. Remove not_implemented guard for ReportFormat::Sarif, wire into match

Out of scope: JUnit/Markdown, config multi-format output, file-level violation locations (no span stored), external schema validator

