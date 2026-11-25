//! Plugin config discovery and command invocation.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{ExternalFinding, ExternalPluginConfig};

/// Load all `.toml` plugin configs from `dir`. Silently skips unreadable or
/// malformed files and returns only valid configs.
pub fn load_plugins(dir: &Path) -> Vec<ExternalPluginConfig> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    entries.flatten().filter_map(load_plugin_config).collect()
}

/// Parse one directory entry into a plugin config; non-TOML entries are
/// skipped silently.
fn load_plugin_config(entry: std::fs::DirEntry) -> Option<ExternalPluginConfig> {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("toml") {
        return None;
    }
    parse_plugin_manifest(&path)
}

/// Parse a `*.toml` manifest; unreadable files are skipped silently,
/// malformed ones are reported to stderr.
fn parse_plugin_manifest(path: &Path) -> Option<ExternalPluginConfig> {
    let source = std::fs::read_to_string(path).ok()?;
    match toml::from_str::<ExternalPluginConfig>(&source) {
        Ok(cfg) => Some(cfg),
        Err(err) => {
            eprintln!(
                "pretender: skipping invalid plugin {:?}: {err}",
                path.file_name().unwrap_or_default()
            );
            None
        }
    }
}

/// Run each plugin against the applicable files and return per-path findings.
pub fn run_plugins(
    plugins: &[ExternalPluginConfig],
    files: &[PathBuf],
) -> BTreeMap<String, Vec<ExternalFinding>> {
    let mut by_path: BTreeMap<String, Vec<ExternalFinding>> = BTreeMap::new();

    for plugin in plugins {
        let Some(findings) = findings_for_plugin(plugin, files) else {
            continue;
        };

        for (path, mut file_findings) in findings {
            by_path.entry(path).or_default().append(&mut file_findings);
        }
    }

    by_path
}

/// Run a single plugin against the files it applies to and parse its output.
///
/// Returns `None` when no file matches or the plugin command failed (the
/// failure is already reported to stderr).
fn findings_for_plugin(
    plugin: &ExternalPluginConfig,
    files: &[PathBuf],
) -> Option<BTreeMap<String, Vec<ExternalFinding>>> {
    let matching: Vec<&PathBuf> = files.iter().filter(|f| applies_to(plugin, f)).collect();
    if matching.is_empty() {
        return None;
    }

    let stdout = invoke_reported(plugin, &matching)?;
    Some(super::parse::parse_with_plugin(plugin, &stdout))
}

/// Run the plugin, reporting failures to stderr as a skipped integration;
/// `None` when the command failed (already reported).
fn invoke_reported(plugin: &ExternalPluginConfig, files: &[&PathBuf]) -> Option<String> {
    match invoke_plugin(plugin, files) {
        Ok(stdout) => Some(stdout),
        Err(err) => {
            eprintln!("pretender: plugin '{}' failed: {err}", plugin.name);
            None
        }
    }
}

pub(super) fn applies_to(plugin: &ExternalPluginConfig, file: &Path) -> bool {
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
        push_command_arg(&mut cmd, arg, files);
    }
    let output = cmd
        .output()
        .with_context(|| format!("failed to launch plugin '{}'", plugin.name))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Append one command argument; the `{files}` placeholder expands to every
/// matched file path, any other argument is appended verbatim.
fn push_command_arg(cmd: &mut Command, arg: &str, files: &[&PathBuf]) {
    if arg != "{files}" {
        cmd.arg(arg);
        return;
    }
    for f in files {
        cmd.arg(f.as_os_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_plugin::ParserKind;

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
        let cfg: ExternalPluginConfig =
            toml::from_str(RUFF_PLUGIN).unwrap_or_else(|e| panic!("parses: {e:?}"));
        assert_eq!(cfg.name, "ruff");
        assert_eq!(cfg.extensions, vec![".py"]);
        assert_eq!(cfg.parser, ParserKind::Json);
        assert_eq!(cfg.mapping.path, "filename");
        assert_eq!(cfg.mapping.line, "location.row");
        assert_eq!(cfg.mapping.code, Some("code".to_string()));
    }

    #[test]
    fn applies_to_matches_extension() {
        let cfg: ExternalPluginConfig =
            toml::from_str(RUFF_PLUGIN).unwrap_or_else(|e| panic!("parses: {e:?}"));
        assert!(applies_to(&cfg, Path::new("foo.py")));
        assert!(!applies_to(&cfg, Path::new("foo.rs")));
        assert!(!applies_to(&cfg, Path::new("foo")));
    }

    #[test]
    fn load_plugins_returns_empty_for_missing_dir() {
        let plugins = load_plugins(Path::new("/nonexistent/path"));
        assert!(plugins.is_empty());
    }

    #[test]
    fn load_plugins_skips_non_toml_files() {
        let dir = tempfile_dir();
        std::fs::write(dir.join("readme.md"), "not toml")
            .unwrap_or_else(|e| panic!("write readme.md: {e:?}"));
        std::fs::write(dir.join("plugin.toml"), RUFF_PLUGIN)
            .unwrap_or_else(|e| panic!("write plugin.toml: {e:?}"));
        let plugins = load_plugins(&dir);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "ruff");
    }

    fn tempfile_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|e| panic!("system clock: {e:?}"))
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pretender-plugin-test-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create {dir:?}: {e:?}"));
        dir
    }
}
