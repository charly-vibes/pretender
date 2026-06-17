# Writing Plugins

pretender supports external metric plugins: small programs that analyse source
files and emit findings in JSON. pretender runs each plugin as a subprocess,
collects its output, and merges the findings into `pretender check` results
alongside the built-in metrics.

---

## Install location

Place plugin manifests (`.toml` files) in the metrics directory. pretender
searches the first directory found in this order:

1. `$PRETENDER_METRICS_DIR` (if set)
2. `$XDG_CONFIG_HOME/pretender/metrics/` (if `XDG_CONFIG_HOME` is set)
3. `~/.config/pretender/metrics/` (fallback)

Each `.toml` file in that directory is loaded as one plugin. Non-`.toml` files
and files with parse errors are skipped — a warning is printed to `stderr` for
each invalid file.

---

## Plugin manifest format

A plugin manifest is a TOML file with the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Plugin name shown in output (e.g. `"ruff"`) |
| `extensions` | array of strings | yes | File extensions this plugin applies to (e.g. `[".py"]`) — must include the leading dot |
| `command` | array of strings | yes | Command and arguments to run. The token `{files}` is expanded to the list of matching file paths |
| `parser` | string | yes | Output format: `"json"` (a JSON array) or `"json-lines"` (one JSON object per line) |
| `[mapping]` | table | yes | Field mapping from the plugin's JSON output to pretender's finding schema (see below) |

### `[mapping]` sub-table

Each mapping value is a **dot-separated path** into the JSON object emitted by
the plugin. For example, `"location.row"` navigates `{"location": {"row": 5}}`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | JSON path to the source file name |
| `line` | string | yes | JSON path to the line number (integer) |
| `message` | string | yes | JSON path to the human-readable finding message |
| `code` | string | no | JSON path to an optional finding code (e.g. `"E501"`) |

### Finding schema

Each finding pretender emits from a plugin has these fields:

| Field | Type | Description |
|-------|------|-------------|
| `source` | string | The plugin `name` |
| `line` | integer | Line number of the finding |
| `message` | string | Human-readable description |
| `code` | string or null | Optional finding code |

---

## How findings are merged

After each `pretender check` run, pretender runs every applicable plugin
against the files being checked. Findings from plugins are merged with built-in
metric findings and displayed in the same output stream. The `source` field
identifies which plugin produced each finding.

Plugins are always loaded from the metrics directory in addition to any
built-in metric tools listed in `[plugins].metrics` in `pretender.toml`.

---

## Worked example

The `metrics/example/` directory in this repository contains a minimal,
runnable plugin: a shell script that emits one JSON finding per invocation,
and a TOML manifest wiring it to pretender.

### Manifest — `metrics/example/long-lines.toml`

```toml
name       = "long-lines"
extensions = [".py", ".js", ".ts", ".rs"]
command    = ["metrics/example/long-lines.sh", "{files}"]
parser     = "json-lines"

[mapping]
path    = "file"
line    = "line"
message = "message"
code    = "code"
```

### Script — `metrics/example/long-lines.sh`

The script checks for lines exceeding 100 characters and emits one JSON object
per violation:

```sh
#!/usr/bin/env sh
for file in "$@"; do
  awk -v file="$file" 'length > 100 {
    printf "{\"file\":\"%s\",\"line\":%d,\"message\":\"line exceeds 100 chars (%d)\",\"code\":\"LL001\"}\n",
      file, NR, length
  }' "$file"
done
```

### Installing the example

The manifest's `command` must be resolvable from wherever `pretender check`
is run. Absolute paths are safest — copy both files to the metrics directory:

```sh
mkdir -p ~/.config/pretender/metrics
cp metrics/example/long-lines.sh   ~/.config/pretender/metrics/
chmod +x ~/.config/pretender/metrics/long-lines.sh

# Edit the command path in the manifest before copying
sed 's|metrics/example/long-lines.sh|~/.config/pretender/metrics/long-lines.sh|' \
  metrics/example/long-lines.toml \
  > ~/.config/pretender/metrics/long-lines.toml

# Verify it appears in check output
pretender check src/
```

The finding will appear in human output as:

```
src/lib.rs:87  [long-lines] line exceeds 100 chars (112)  LL001
```

---

## Real-world examples

### ruff (Python linter)

```toml
name       = "ruff"
extensions = [".py"]
command    = ["ruff", "check", "--output-format=json", "--select=E501", "{files}"]
parser     = "json"

[mapping]
path    = "filename"
line    = "location.row"
message = "message"
code    = "code"
```

### ESLint (JavaScript / TypeScript)

ESLint's JSON output nests messages inside per-file objects — one array entry
per file, not per finding. pretender's dot-path mapping does not support array
indexing, so a thin wrapper script is required to flatten the output:

```sh
#!/usr/bin/env sh
# eslint-wrapper.sh
# Requires jq. Flattens ESLint JSON output to one JSON object per finding.
npx eslint --format=json "$@" | jq -c '
  .[] | .filePath as $f | .messages[] |
  {file: $f, line: .line, message: .message, code: .ruleId}
'
```

```toml
name       = "eslint"
extensions = [".js", ".jsx", ".ts", ".tsx"]
command    = ["/path/to/eslint-wrapper.sh", "{files}"]
parser     = "json-lines"

[mapping]
path    = "file"
line    = "line"
message = "message"
code    = "code"
```

Replace `/path/to/eslint-wrapper.sh` with the absolute path where you install
the wrapper script.
