use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalPluginConfig {
    pub name: String,
    pub extensions: Vec<String>,
    pub command: Vec<String>,
    pub parser: ParserKind,
    pub mapping: FieldMapping,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ParserKind {
    Json,
    JsonLines,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldMapping {
    pub path: String,
    pub line: String,
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalFinding {
    pub source: String,
    pub line: u32,
    pub message: String,
    pub code: Option<String>,
}

/// Returns the directory for external metric plugin configs.
///
/// Priority: `PRETENDER_METRICS_DIR` env var → `$XDG_CONFIG_HOME/pretender/metrics`
/// → `~/.config/pretender/metrics`.
pub fn default_metrics_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PRETENDER_METRICS_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("pretender").join("metrics")
}

/// Load all `.toml` plugin configs from `dir`. Silently skips unreadable or
/// malformed files and returns only valid configs.
pub fn load_plugins(dir: &Path) -> Vec<ExternalPluginConfig> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut plugins = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        match toml::from_str::<ExternalPluginConfig>(&source) {
            Ok(cfg) => plugins.push(cfg),
            Err(err) => eprintln!(
                "pretender: skipping invalid plugin {:?}: {err}",
                path.file_name().unwrap_or_default()
            ),
        }
    }
    plugins
}

/// Run each plugin against the applicable files and return per-path findings.
pub fn run_plugins(
    plugins: &[ExternalPluginConfig],
    files: &[PathBuf],
) -> BTreeMap<String, Vec<ExternalFinding>> {
    let mut by_path: BTreeMap<String, Vec<ExternalFinding>> = BTreeMap::new();

    for plugin in plugins {
        let matching: Vec<&PathBuf> = files.iter().filter(|f| applies_to(plugin, f)).collect();
        if matching.is_empty() {
            continue;
        }

        let stdout = match invoke_plugin(plugin, &matching) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("pretender: plugin '{}' failed: {err}", plugin.name);
                continue;
            }
        };

        let findings = match plugin.parser {
            ParserKind::Json => parse_json_array(&stdout, &plugin.name, &plugin.mapping),
            ParserKind::JsonLines => parse_json_lines(&stdout, &plugin.name, &plugin.mapping),
        };

        for (path, mut file_findings) in findings {
            by_path.entry(path).or_default().append(&mut file_findings);
        }
    }

    by_path
}

fn applies_to(plugin: &ExternalPluginConfig, file: &Path) -> bool {
    let ext = file
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    plugin.extensions.contains(&ext)
}

fn invoke_plugin(plugin: &ExternalPluginConfig, files: &[&PathBuf]) -> Result<String> {
    let mut cmd = Command::new(&plugin.command[0]);
    for arg in &plugin.command[1..] {
        if arg == "{files}" {
            for f in files {
                cmd.arg(f.as_os_str());
            }
        } else {
            cmd.arg(arg);
        }
    }
    let output = cmd
        .output()
        .with_context(|| format!("failed to launch plugin '{}'", plugin.name))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_json_array(
    stdout: &str,
    source: &str,
    mapping: &FieldMapping,
) -> BTreeMap<String, Vec<ExternalFinding>> {
    let mut result: BTreeMap<String, Vec<ExternalFinding>> = BTreeMap::new();
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(stdout) else {
        return result;
    };
    for value in values {
        if let Some((path, finding)) = extract_finding(&value, source, mapping) {
            result.entry(path).or_default().push(finding);
        }
    }
    result
}

fn parse_json_lines(
    stdout: &str,
    source: &str,
    mapping: &FieldMapping,
) -> BTreeMap<String, Vec<ExternalFinding>> {
    let mut result: BTreeMap<String, Vec<ExternalFinding>> = BTreeMap::new();
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some((path, finding)) = extract_finding(&value, source, mapping) {
            result.entry(path).or_default().push(finding);
        }
    }
    result
}

fn extract_finding(
    value: &serde_json::Value,
    source: &str,
    mapping: &FieldMapping,
) -> Option<(String, ExternalFinding)> {
    let path = get_nested(value, &mapping.path)?.as_str()?.to_string();
    let line = get_nested(value, &mapping.line)?.as_u64()? as u32;
    let message = get_nested(value, &mapping.message)?.as_str()?.to_string();
    let code = mapping
        .code
        .as_deref()
        .and_then(|p| get_nested(value, p))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Some((
        path,
        ExternalFinding {
            source: source.to_string(),
            line,
            message,
            code,
        },
    ))
}

/// Resolve a dot-separated path through a JSON value.
fn get_nested<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUFF_PLUGIN: &str = r#"
name = "ruff"
extensions = [".py"]
command = ["ruff", "check", "--output-format=json", "--select=E501", "{files}"]
parser = "json"

[mapping]
path = "filename"
line = "location.row"
message = "message"
code = "code"
"#;

    #[test]
    fn parses_plugin_config() {
        let cfg: ExternalPluginConfig = toml::from_str(RUFF_PLUGIN).expect("parses");
        assert_eq!(cfg.name, "ruff");
        assert_eq!(cfg.extensions, vec![".py"]);
        assert_eq!(cfg.parser, ParserKind::Json);
        assert_eq!(cfg.mapping.path, "filename");
        assert_eq!(cfg.mapping.line, "location.row");
        assert_eq!(cfg.mapping.code, Some("code".to_string()));
    }

    #[test]
    fn applies_to_matches_extension() {
        let cfg: ExternalPluginConfig = toml::from_str(RUFF_PLUGIN).expect("parses");
        assert!(applies_to(&cfg, Path::new("foo.py")));
        assert!(!applies_to(&cfg, Path::new("foo.rs")));
        assert!(!applies_to(&cfg, Path::new("foo")));
    }

    #[test]
    fn parse_json_array_extracts_findings() {
        let json = r#"[
            {"filename": "foo.py", "location": {"row": 3, "column": 1}, "message": "Line too long", "code": "E501"},
            {"filename": "bar.py", "location": {"row": 7, "column": 1}, "message": "Unused import", "code": "F401"}
        ]"#;
        let mapping = FieldMapping {
            path: "filename".to_string(),
            line: "location.row".to_string(),
            message: "message".to_string(),
            code: Some("code".to_string()),
        };
        let result = parse_json_array(json, "ruff", &mapping);
        let foo = result.get("foo.py").expect("foo.py findings");
        assert_eq!(foo.len(), 1);
        assert_eq!(foo[0].source, "ruff");
        assert_eq!(foo[0].line, 3);
        assert_eq!(foo[0].code.as_deref(), Some("E501"));
        let bar = result.get("bar.py").expect("bar.py findings");
        assert_eq!(bar[0].message, "Unused import");
    }

    #[test]
    fn parse_json_array_ignores_malformed_entries() {
        let json = r#"[
            {"filename": "foo.py", "location": {"row": 1}, "message": "ok", "code": "E501"},
            {"bad": "entry"},
            null
        ]"#;
        let mapping = FieldMapping {
            path: "filename".to_string(),
            line: "location.row".to_string(),
            message: "message".to_string(),
            code: Some("code".to_string()),
        };
        let result = parse_json_array(json, "ruff", &mapping);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_json_lines_extracts_findings() {
        let lines = concat!(
            "{\"file\": \"a.py\", \"row\": 5, \"msg\": \"err\"}\n",
            "{\"file\": \"a.py\", \"row\": 9, \"msg\": \"warn\"}\n",
            "not-json\n",
        );
        let mapping = FieldMapping {
            path: "file".to_string(),
            line: "row".to_string(),
            message: "msg".to_string(),
            code: None,
        };
        let result = parse_json_lines(lines, "tool", &mapping);
        let findings = result.get("a.py").expect("a.py");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].line, 5);
        assert!(findings[0].code.is_none());
    }

    #[test]
    fn get_nested_resolves_dotted_path() {
        let value = serde_json::json!({"location": {"row": 42}});
        let result = get_nested(&value, "location.row");
        assert_eq!(result.and_then(|v| v.as_u64()), Some(42));
    }

    #[test]
    fn get_nested_returns_none_for_missing_key() {
        let value = serde_json::json!({"a": 1});
        assert!(get_nested(&value, "b.c").is_none());
    }

    #[test]
    fn load_plugins_returns_empty_for_missing_dir() {
        let plugins = load_plugins(Path::new("/nonexistent/path"));
        assert!(plugins.is_empty());
    }

    #[test]
    fn load_plugins_skips_non_toml_files() {
        let dir = tempfile_dir();
        std::fs::write(dir.join("readme.md"), "not toml").unwrap();
        std::fs::write(dir.join("plugin.toml"), RUFF_PLUGIN).unwrap();
        let plugins = load_plugins(&dir);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "ruff");
    }

    fn tempfile_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pretender-plugin-test-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
