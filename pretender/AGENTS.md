## Structural-clarity ticket contract

Every structural-clarity ticket follows the 8-step contract in
[docs/our-standard.md](../docs/our-standard.md): research, design, RED
(un-grandfather baseline IDs via `just dogfood-red`), GREEN, TIDY,
independent review, mechanical gates green, ledger update. The baseline
(`pretender-baseline.json`) is the burn-down ledger of grandfathered
findings: additions are forbidden (baseline-monotonicity gate), it may
only shrink inside the epic, and only ticket 5zo.15 (the ratchet)
re-adopts IDs, and only after thresholds tighten. File new tickets from
`.beads/templates/ticket.md`.
