---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:green]
---

GREEN: SARIF 2.1.0 via serde-sarif builder API. Added start_line to UnitReport from unit.span.start_line. write_sarif_report() iterates file>unit>violation, deduplicates rules by metric name, builds locations with artifactLocation.uri + region.startLine. Used python_violator.py fixture (exceeds default cyclomatic_max=10) to ensure violations exist without project-level pretender.toml.
