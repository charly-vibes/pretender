//! Mapping plugin stdout (JSON array or JSON lines) to per-path findings.

use std::collections::BTreeMap;

use super::{ExternalFinding, ExternalPluginConfig, FieldMapping, ParserKind};

/// Parse `stdout` with the plugin's configured parser.
pub(super) fn parse_with_plugin(
    plugin: &ExternalPluginConfig,
    stdout: &str,
) -> BTreeMap<String, Vec<ExternalFinding>> {
    match plugin.parser {
        ParserKind::Json => parse_json_array(stdout, &plugin.name, &plugin.mapping),
        ParserKind::JsonLines => parse_json_lines(stdout, &plugin.name, &plugin.mapping),
    }
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
    let code = optional_code(value, mapping);
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

/// The optional `code` field: looked up only when the mapping names it.
fn optional_code(value: &serde_json::Value, mapping: &FieldMapping) -> Option<String> {
    mapping
        .code
        .as_deref()
        .and_then(|p| get_nested(value, p))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
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

    fn ruff_mapping() -> FieldMapping {
        FieldMapping {
            path: "filename".to_string(),
            line: "location.row".to_string(),
            message: "message".to_string(),
            code: Some("code".to_string()),
        }
    }

    fn lines_mapping() -> FieldMapping {
        FieldMapping {
            path: "file".to_string(),
            line: "row".to_string(),
            message: "msg".to_string(),
            code: None,
        }
    }

    #[test]
    fn parse_json_array_extracts_findings() {
        let json = r#"[
            {"filename": "foo.py", "location": {"row": 3, "column": 1}, "message": "Line too long", "code": "E501"},
            {"filename": "bar.py", "location": {"row": 7, "column": 1}, "message": "Unused import", "code": "F401"}
        ]"#;
        let result = parse_json_array(json, "ruff", &ruff_mapping());
        let foo = result
            .get("foo.py")
            .unwrap_or_else(|| panic!("foo.py findings"));
        assert_eq!(foo.len(), 1);
        assert_eq!(foo[0].source, "ruff");
        assert_eq!(foo[0].line, 3);
        assert_eq!(foo[0].code.as_deref(), Some("E501"));
        let bar = result
            .get("bar.py")
            .unwrap_or_else(|| panic!("bar.py findings"));
        assert_eq!(bar[0].message, "Unused import");
    }

    #[test]
    fn parse_json_array_ignores_malformed_entries() {
        let json = r#"[
            {"filename": "foo.py", "location": {"row": 1}, "message": "ok", "code": "E501"},
            {"bad": "entry"},
            null
        ]"#;
        let result = parse_json_array(json, "ruff", &ruff_mapping());
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_json_lines_extracts_findings() {
        let lines = concat!(
            "{\"file\": \"a.py\", \"row\": 5, \"msg\": \"err\"}\n",
            "{\"file\": \"a.py\", \"row\": 9, \"msg\": \"warn\"}\n",
            "not-json\n",
        );
        let result = parse_json_lines(lines, "tool", &lines_mapping());
        let findings = result
            .get("a.py")
            .unwrap_or_else(|| panic!("a.py findings"));
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
}
