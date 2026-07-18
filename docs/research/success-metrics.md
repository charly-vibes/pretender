# Pretender Adoption Success Metrics

## Baselines (2026-07-18)

| Metric | Current Value | Target (2 months) | Stretch |
|--------|---------------|-------------------|---------|
| Non-pretender project invocations (pi logs) | 14 | 30 | 50 |
| Projects with `pretender.toml` | 5 | 8 | 12 |
| Projects with pretender in justfile | 5 | 10 | 15 |
| Projects with pretender in CI | 3 | 6 | 10 |
| Projects with CLAUDE.md/AGENTS.md pretender mention | 1 | 5 | 8 |
| `wai way` shows pretender "pass" | 0 | All charly projects | All projects |

## Measurement Cadence

- Record metrics monthly
- First review: 2026-09-18
- Archive at 2027-01-18 (6 months) or when targets are met

## How to Measure

### Invocations
```bash
cd /var/home/sasha/.pi/agent/sessions
find . -name "*.jsonl" -not -path "*/pretender--/*" -exec grep -l "pretender" {} \; | wc -l
```

### Config files
```bash
find /var/home/sasha/para/areas/dev/gh/{charly,sk}/*/pretender.toml -maxdepth 1 2>/dev/null | wc -l
```

### justfile references
```bash
grep -r "pretender" /var/home/sasha/para/areas/dev/gh/{charly,sk}/*/justfile 2>/dev/null | wc -l
```

### CLAUDE.md / AGENTS.md mentions
```bash
grep -r "pretender" /var/home/sasha/para/areas/dev/gh/{charly,sk}/*/{CLAUDE.md,AGENTS.md,claude.md} 2>/dev/null | wc -l
```
