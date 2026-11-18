# Project status brief

## Purpose / current state

none

## Focus

none

## In flight

- pretender-nvn — docs: apply README front-door standard v0.4 (§5.1) (status: open; priority: 1; age: 2d)
- pretender-4cs — Terminate hanging or noisy external scanners within configured budgets (status: open; priority: 2; age: 40d; blocked by: pretender-fqk)
- pretender-6tt — Rust extractor does not populate params metric — params_max unenforced for Rust (status: open; priority: 2; age: 14d)
- pretender-fqk — Distinguish external scanner failures from clean quality results (status: open; priority: 2; age: 40d)
- pretender-gv5 — void_mutator detection is nondeterministic across scans — gate 'known' count flaps 55↔49 on an unchanged tree (status: open; priority: 2; age: 14d)
- pretender-t1b — Deployed spec drift: reserved-command and reserved-staged scenarios contradict shipped behavior (status: open; priority: 2; age: 14d)
- pretender-2uv — openspec 1.x upgrade blocked: 10 validation failures under @fission-ai/openspec@1.14.2 (status: open; priority: 3; age: 33d)
- pretender-c60 — Configure cargo runner in .espectacular + reconcile contract test entries (status: open; priority: 3; age: 14d)

## Blocked / waiting

- pretender-4cs — Terminate hanging or noisy external scanners within configured budgets (status: open; priority: 2; age: 40d; blocked by: pretender-fqk)

## Next / ready

- pretender-nvn — docs: apply README front-door standard v0.4 (§5.1) (status: open; priority: 1; age: 2d)
- pretender-6tt — Rust extractor does not populate params metric — params_max unenforced for Rust (status: open; priority: 2; age: 14d)
- pretender-fqk — Distinguish external scanner failures from clean quality results (status: open; priority: 2; age: 40d)
- pretender-gv5 — void_mutator detection is nondeterministic across scans — gate 'known' count flaps 55↔49 on an unchanged tree (status: open; priority: 2; age: 14d)
- pretender-t1b — Deployed spec drift: reserved-command and reserved-staged scenarios contradict shipped behavior (status: open; priority: 2; age: 14d)
(2 more omitted)

## Parked

none

### Needs re-triage

none

## Recent

- 18a76a2d31553eeb1c7e7b4cb48dc18b08a1536d beads: close pretender-1ap (2026-11-18T16:08:16Z)
- 5b78ac8836d1a3eeb24f5f5dc13cbc489ebe7743 chore(gates): render the managed gate blocks from .ddl/config.toml (2026-11-18T16:02:42Z)
- 72e18ed8c5e1693ad2f7addf688ee7932cdbd6dc fix(init): stop writing AGENTS.md regions pretender does not own (2026-11-18T15:53:08Z)
- 58450f1e54104240ef3b1a4ae96f8df2a4af06f1 chore(openspec): migrate project.md into config.yaml; pin 2.1.0 (2026-11-18T15:02:32Z)
- a793212554282fb382fb58f4bfe5491756a9e377 chore(openspec): adopt the 2.1 artifact layout (2026-11-18T15:02:26Z)
(195 more omitted)

## Health + confidence

Health: **amber** — basis: signals — owner: none — reason: orphan_commits: 18 commits in the 14-day window link no tracker id or open work-item path: 036b4b38b71797882cf5c4d19743d338d7e9d2e7 (Merge pull request #53 from charly-vibes/feat/update-app-threshold-defaults); 0a89aada947a20b529c3f0c60c9576ddf552cea5 (chore(beads): file void_mutator nondeterminism observed from tambor gate runs); 49b10e06778702e6e4ad9cb4ce64a57e79d744c7 (Merge pull request #52 from charly-vibes/proposal/update-app-threshold-defaults); 532f346c9f59e9b3a62875680840476868da3d2e (ci(release): keep tap/scoop publishing when crates.io upload fails); 58450f1e54104240ef3b1a4ae96f8df2a4af06f1 (chore(openspec): migrate project.md into config.yaml; pin 2.1.0); 5b78ac8836d1a3eeb24f5f5dc13cbc489ebe7743 (chore(gates): render the managed gate blocks from .ddl/config.toml); 5e8e5cabd1cb1845fa8292788a253c869623fda5 (beads: file the foreign-region overwrite defect); 6b60acd39afa1d68820942a402ba9e2b9e417117 (docs: apply issue-review fixes to README/release tickets); 6f39b674401db4d8411f5abb7c9f93d7df51158e (chore(release): v0.11.1 — dot-prefixed exclude globs, pending-commit diff mode); 72e18ed8c5e1693ad2f7addf688ee7932cdbd6dc (fix(init): stop writing AGENTS.md regions pretender does not own); 856824097a5c190281fa50efeaa6d4c871f2bc02 (fix(gate): match dot-prefixed walked paths against exclude globs; cover worktree in diff mode); 86a91a7f15b774bf2e4da495c3eed995b62cfe35 (feat(config): tighten threshold defaults, add init threshold presets); 972f4fa2fcd8c504372808543846dda23442de50 (docs: file README front-door and release tickets); a793212554282fb382fb58f4bfe5491756a9e377 (chore(openspec): adopt the 2.1 artifact layout); c229b2990dd20aed4b4101811b6aa26fbb0be191 (fix(gates): unblock pre-commit — contract adoption, spk gate retarget, baseline); cf49a70cabf228778d1fe390d5b532812c6091bf (fix(init): stop injecting the retired OPENSPEC block; ignore build output); f0a0a106c88d8ff5726819abbe25ef7dd5a0d922 (chore(release): v0.11.0); f3fa56a1fa95a222b5a22fde8f5700a8dd3a660b (docs: apply issue-review fixes to README/release tickets); spec_change_without_bead: openspec changes link no tracker id: `add-data-model-checks` changed 0 days ago, `adopt-genesis` changed 0 days ago, `update-app-threshold-defaults` changed 14 days ago
- signal: orphan commits — 18 commits in the 14-day window link no tracker id or open work-item path: 036b4b38b71797882cf5c4d19743d338d7e9d2e7 (Merge pull request #53 from charly-vibes/feat/update-app-threshold-defaults); 0a89aada947a20b529c3f0c60c9576ddf552cea5 (chore(beads): file void_mutator nondeterminism observed from tambor gate runs); 49b10e06778702e6e4ad9cb4ce64a57e79d744c7 (Merge pull request #52 from charly-vibes/proposal/update-app-threshold-defaults); 532f346c9f59e9b3a62875680840476868da3d2e (ci(release): keep tap/scoop publishing when crates.io upload fails); 58450f1e54104240ef3b1a4ae96f8df2a4af06f1 (chore(openspec): migrate project.md into config.yaml; pin 2.1.0); 5b78ac8836d1a3eeb24f5f5dc13cbc489ebe7743 (chore(gates): render the managed gate blocks from .ddl/config.toml); 5e8e5cabd1cb1845fa8292788a253c869623fda5 (beads: file the foreign-region overwrite defect); 6b60acd39afa1d68820942a402ba9e2b9e417117 (docs: apply issue-review fixes to README/release tickets); 6f39b674401db4d8411f5abb7c9f93d7df51158e (chore(release): v0.11.1 — dot-prefixed exclude globs, pending-commit diff mode); 72e18ed8c5e1693ad2f7addf688ee7932cdbd6dc (fix(init): stop writing AGENTS.md regions pretender does not own); 856824097a5c190281fa50efeaa6d4c871f2bc02 (fix(gate): match dot-prefixed walked paths against exclude globs; cover worktree in diff mode); 86a91a7f15b774bf2e4da495c3eed995b62cfe35 (feat(config): tighten threshold defaults, add init threshold presets); 972f4fa2fcd8c504372808543846dda23442de50 (docs: file README front-door and release tickets); a793212554282fb382fb58f4bfe5491756a9e377 (chore(openspec): adopt the 2.1 artifact layout); c229b2990dd20aed4b4101811b6aa26fbb0be191 (fix(gates): unblock pre-commit — contract adoption, spk gate retarget, baseline); cf49a70cabf228778d1fe390d5b532812c6091bf (fix(init): stop injecting the retired OPENSPEC block; ignore build output); f0a0a106c88d8ff5726819abbe25ef7dd5a0d922 (chore(release): v0.11.0); f3fa56a1fa95a222b5a22fde8f5700a8dd3a660b (docs: apply issue-review fixes to README/release tickets)
- signal: spec change without bead — openspec changes link no tracker id: `add-data-model-checks` changed 0 days ago, `adopt-genesis` changed 0 days ago, `update-app-threshold-defaults` changed 14 days ago

## Sources & freshness

| source | status | observed_at | staleness | detail |
| --- | --- | --- | --- | --- |
| beads | present | 2026-11-18T16:06:39Z | fresh | 12 malformed line(s) skipped in .beads/issues.jsonl |
| git | present | 2026-11-18T16:08:16Z | fresh | none |
| openspec | present | 2026-11-18T19:56:14Z | fresh | none |
| whisper | present | 2026-11-04T19:55:25Z | fresh | none |
| curation | present | 2026-11-18T21:12:08Z | fresh | none |

Curation: updated_at 2026-11-18T21:12:08Z; confirmed_at none.

Freshness: generated_at 2026-11-18T21:12:08Z; per-source observed_at and staleness are listed above.
