# pretender

Pretender is a Rust CLI for structural code-quality checks across multiple languages.

## Current MVP

Implemented commands:
- `pretender init`
- `pretender check <paths...>`
- `pretender complexity <path>`
- `pretender report`
- `pretender hooks install|uninstall`
- `pretender ci generate github`

Reserved commands:
- `pretender duplication`
- `pretender mutation`
- `pretender plugins ...`
- `pretender explain <metric>`

## Check output

`pretender check` currently supports:
- `--format human|json|sarif`
- `--output <path>`
- `--mode guidance|tiered|gate`

Not yet implemented:
- `--staged`
- `--diff-only`
- `--diff-base`
- `--format junit|markdown`

## Report output

`pretender report` renders the last successful `pretender check` as:
- `human`
- `markdown`
- `html`

## Development

```bash
just build
just test
just lint
just ci
```
