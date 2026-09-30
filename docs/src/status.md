# Status

**beta** — core works and is dogfooded in anger; surface may shift before stable.

## Implemented

| Command | Status | Notes |
|---|---|---|
| `pretender init` | stable | project config bootstrap |
| `pretender report` | stable | complexity report (cyclomatic, cognitive, abc, params) |
| `pretender ci generate github` | stable | emit CI quality-check steps |
| `pretender lint` | beta | complexity/lint gate over changed code |

## In progress

- Theme/docs conformance round (DDL-u8x epic); docs/src restructure.

## Mapped to specs

- Complexity model definitions in `docs/` (mutation.md, plugins.md); openspec proposals in-repo.

## Dogfooding

- **wai**, **testaruda**, **dont** (planned pins) install pretender per `versions.ddl.toml`; testaruda carries a `pretender.toml`
- pretender's own CI runs `just ci`
