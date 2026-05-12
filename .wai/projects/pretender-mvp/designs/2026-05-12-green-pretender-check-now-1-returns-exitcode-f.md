---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-81c-cli-check-command, pipeline-step:green]
---

GREEN: pretender check now (1) returns ExitCode::FAILURE when any unit has a violation, (2) accepts --output <path> routing report through Box<dyn Write> (stdout otherwise), (3) lists violations in human output with red ANSI gated on tty + !NO_COLOR + writing-to-stdout, (4) processes files via rayon par_iter with post-sort by path for deterministic ordering. New python_violator.py fixture; tests stage fixtures into temp dirs so default app role applies.
