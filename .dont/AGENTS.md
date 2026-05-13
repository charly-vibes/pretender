# dont

This project uses `dont` for epistemic claim tracking.

For full documentation see the [dont spec](https://github.com/charly-vibes/dont).

## Quick start

```
dont ground "claim text" --file README.md --lines 10-18 # fast path: claim + repo evidence
dont conclude "claim text"                            # introduce an unverified claim
dont trust <id> --reason ...                          # register doubt
dont dismiss <id> --evidence ...                      # verify with evidence
dont show <id>                                        # inspect a claim
dont trace <id>                                       # diagnose blocker paths
dont list                                             # list all claims
```

`dont ground` is the preferred fast path when you already have the claim and evidence in hand. The underlying model is still conclude → trust → dismiss → forget: `ground` composes `conclude` and `dismiss` rather than bypassing the core lifecycle.

## Grounding claims in repository evidence

Prefer repository-relative file locators over opaque `file://` URIs when the evidence lives inside the current project:

```
# Preferred: repository-relative locator
dont ground "documented project fact" --file README.md --lines 10-18
dont dismiss <id> --file src/lib.rs --lines 42-55 --anchor "MyTrait"
dont dismiss <id> --file docs/spec.md --excerpt "The system SHALL..."

# Supported for compatibility: plain URI
dont dismiss <id> --evidence https://external-source.example/ref
```

Repository-relative locators resolve against the project root regardless of the caller's working directory. Paths that escape the project root (via `..` traversal or symlink escape) are refused.

When `show` or `why` reports stale, unresolved, or otherwise confusing blockers, run `dont trace <id>` to see the blocker path that explains what dependency or support fallout needs attention.
