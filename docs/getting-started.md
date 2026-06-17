# Getting Started

This guide walks you from a fresh install to a passing `pretender check` in
a real repository. It covers the six steps most teams need on day one.

---

## 1. Install

```sh
cargo install pretender
```

Verify the install:

```sh
pretender --version
```

---

## 2. Initialise your repository

Run the interactive wizard from the root of your repository:

```sh
pretender init
```

The wizard will:

1. Ask which languages your project uses
2. Suggest threshold values based on project size
3. Write `pretender.toml` to the repo root
4. Offer to install the pre-commit hook
5. Offer to generate a CI workflow file

A minimal generated `pretender.toml` looks like:

```toml
[pretender]
mode = "tiered"
languages = ["auto"]

[thresholds]
cyclomatic_max = 10
cognitive_max  = 15
```

See the [Configuration reference](configuration.md) for every available key.

---

## 3. Run your first check

```sh
pretender check .
```

`pretender check .` scans all files in the given paths. To check only files
changed relative to `origin/main`:

```sh
pretender check . --diff-only
```

### Reading the output

```
src/parser.py  cyclomatic=14  [threshold: 10]  ● HIGH
  ↳ parse_token()  cyclomatic=14

src/utils.py   cyclomatic=6   cognitive=8      ✓ OK
```

| Symbol | Meaning |
|--------|---------|
| `✓ OK` | All metrics within thresholds |
| `● HIGH` | One or more metrics exceed the red band |
| `◐ MEDIUM` | Metrics in the yellow band |
| `○ LOW` | Guidance-only hint |

Severity is determined by the `[bands]` thresholds in `pretender.toml`. In
`gate` mode every violation fails the run regardless of band.

---

## 4. Fix a violation

Suppose `parse_token()` scores `cyclomatic=14` against a threshold of 10.
The most direct fix is to extract branches into helper functions:

**Before** (cyclomatic=14):

```python
def parse_token(token, context, strict):
    if token.type == "string":
        if strict and not token.value.startswith('"'):
            raise ValueError("unquoted string")
        return token.value.strip('"')
    elif token.type == "number":
        try:
            return int(token.value)
        except ValueError:
            return float(token.value)
    # … six more branches
```

**After** (cyclomatic≈3 each):

```python
def _parse_string(token, strict):
    if strict and not token.value.startswith('"'):
        raise ValueError("unquoted string")
    return token.value.strip('"')

def _parse_number(token):
    try:
        return int(token.value)
    except ValueError:
        return float(token.value)

def parse_token(token, context, strict):
    if token.type == "string":
        return _parse_string(token, strict)
    elif token.type == "number":
        return _parse_number(token)
    # …
```

Recheck after refactoring:

```sh
pretender check src/parser.py
```

---

## 5. Install the pre-commit hook

Keep violations from reaching the repository by running pretender before
each commit:

```sh
pretender hooks install
```

This writes a `.git/hooks/pre-commit` script that runs `pretender check
--staged` on the files you are about to commit. To remove it:

```sh
pretender hooks uninstall
```

---

## 6. Add CI integration

Generate a GitHub Actions workflow:

```sh
pretender ci generate github
```

This writes `.github/workflows/pretender.yml`. The workflow runs
`pretender check` on every pull request and, if mutation testing is
configured, gates merges on the mutation score defined by
`thresholds.mutation_min` in your `pretender.toml`.

For mutation testing details, see [Mutation testing](mutation.md).

---

## Next steps

| Topic | Document |
|-------|----------|
| All `pretender.toml` keys and defaults | [Configuration reference](configuration.md) |
| Mutation testing flags and JSON output | [Mutation testing](mutation.md) |
| Writing custom metric plugins | [Writing plugins](plugins.md) |
| Language-specific tracked nodes | [languages/](../languages/) |
