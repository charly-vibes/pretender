# Indirect Pretender Usage Audit

Results of the investigation into indirect pretender usage across all
charly and sk projects.

## Summary

| Category | Count | Notes |
|----------|-------|-------|
| Projects with `pretender.toml` | 5 | testaruda, cositos, smithy, Testimonial.jl, Tray.jl |
| Projects with pretender in justfile | 5 | pretender (self), testaruda, smithy, Testimonial.jl, Tray.jl |
| Projects with pretender in CI | 3 (non-pretender) | cositos, smithy, Testimonial.jl |
| Projects with pre-commit hooks | 3 | smithy, Testimonial.jl, Tray.jl |
| Projects with CLAUDE.md mention | 1 | cositos |
| Direct invocations (non-pretender) | 14 | From pi session logs |
| Indirect invocations (pi logs) | 0 | No `just complexity` calls found in logs |

## Corrected Total Usage

- **Direct invocations**: 55 total (41 from pretender's own development, **14 from other projects**)
- **Projects using pretender (config or CI or hooks)**: 5 distinct projects
- **Indirect pi session usage**: 0 — agents don't invoke justfile wrappers that wrap pretender

## Conclusion

The indirect usage is minimal. The 5 projects with pretender integration
represent the true adoption footprint. No hidden usage was found.
