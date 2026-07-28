# cli-and-config spec delta: adopt genesis

## MODIFIED Requirements

### Requirement: Output Formats

pretender's `--format json` output SHALL wrap every payload in `genesis::envelope::Envelope` so its JSON shape matches wai/dont/espectacular/testaruda across the suite.

#### Scenario: check emits shared envelope

- **WHEN** `pretender check --format json` is run after adopting genesis
- **THEN** the emitted JSON SHALL have top-level keys `ok`, `envelope_version`, `cli_version`, `envelope_kind`, `data`, `warnings`, `hints`, `meta`
- **AND** the per-file findings array SHALL be nested under `data`.

### Requirement: Init Command

`pretender init` SHALL inject suite managed blocks (`<!-- WAI:START -->`, `<!-- OPENSPEC:START -->`, `<!-- DONT:START -->`) via `genesis::managed_block`, so pretender participates in the suite's AGENTS.md managed-block system like wai/dont/espectacular.

#### Scenario: init injects managed blocks

- **WHEN** `pretender init` is run in a fresh repo
- **THEN** `AGENTS.md` SHALL contain the WAI/OPENSPEC/DONT managed blocks
- **AND** the injector mechanics SHALL come from `genesis::managed_block`, not local code.

## ADDED Requirements

### Requirement: feedback subcommand

pretender SHALL provide a `feedback` subcommand that files a structured issue against pretender's upstream repo via `gh`, wrapping `genesis::feedback`. The `report` verb is unchanged and keeps its "render local check output" meaning.

#### Scenario: agent files a bug with last error

- **WHEN** `pretender feedback bug --from-last-error --yes` is run after a non-zero exit
- **THEN** pretender SHALL read its own error scratch
- **AND** SHALL assemble and redact the body via `genesis::feedback`
- **AND** SHALL invoke `gh issue create` against pretender's `Cargo.toml` `repository` with labels `agent-reported`, `bug`, `has-repro`.

#### Scenario: report verb is unchanged

- **WHEN** `pretender report --format json` is run
- **THEN** it SHALL render the last check output as before (the `report` verb is NOT repurposed for issue filing).