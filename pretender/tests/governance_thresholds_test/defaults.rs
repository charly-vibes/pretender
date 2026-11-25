//! Extraction of thresholds-struct defaults from pretender's config module
//! sources: locates each table's `impl Default for <Struct>` block and
//! parses its numeric and boolean field defaults.
//!
//! # Responsibilities
//! Map floor-file tables to their thresholds structs (including flattened
//! sub-struct groups) and extract `impl Default` field literals.
//!
//! # Rationale
//! Split out of `floor_gate.rs` so both files stay within the gate's
//! per-file size limit; bodies moved verbatim.

use std::collections::BTreeMap;

pub(crate) const TABLE_STRUCTS: &[(&str, &str)] = &[
    ("app", "AppThresholds"),
    ("test", "TestThresholds"),
    ("library", "LibraryThresholds"),
    ("script", "ScriptThresholds"),
    ("coupling", "CouplingThresholds"),
    ("unit-test", "DurationThresholds"),
    ("integration-test", "DurationThresholds"),
];

/// Flattened sub-structs whose `impl Default` carries part of a table's
/// leaf defaults (wave 4 ratchet split the two oversized tables; the TOML
/// spelling is unchanged, so the bounds here are unchanged too).
pub(crate) const TABLE_GROUPS: &[(&str, &[&str])] = &[
    ("app", &["AppChecksThresholds"]),
    ("test", &["TestChecksThresholds"]),
];

pub(crate) fn struct_name(table: &str) -> &'static str {
    TABLE_STRUCTS
        .iter()
        .find(|(name, _)| *name == table)
        .copied()
        .map(|(_, struct_name)| struct_name)
        .unwrap_or("")
}

/// Numeric default values of one thresholds struct, extracted from its
/// `impl Default` block in src/config.rs.
#[derive(Default)]
pub(crate) struct Defaults {
    pub(crate) nums: BTreeMap<String, f64>,
    pub(crate) bools: BTreeMap<String, bool>,
}

fn impl_block<'a>(source: &'a str, struct_name: &str) -> &'a str {
    let marker = format!("impl Default for {struct_name} {{");
    let Some(start) = source.find(&marker) else {
        return "";
    };
    let body = &source[start..];
    let Some(end) = body.find("\n    }\n") else {
        return body;
    };
    &body[..end]
}

fn split_field(line: &str) -> Option<(String, String)> {
    let (key, value) = line.trim().split_once(':')?;
    let value = value.trim().trim_end_matches(',').trim().to_string();
    Some((key.trim().to_string(), value))
}

/// First numeric literal in the raw field value ("Some(1)," -> 1).
fn parse_number(raw: &str) -> Option<f64> {
    let start = raw.find(|ch: char| ch.is_ascii_digit())?;
    let tail = &raw[start..];
    let end = tail
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .unwrap_or(tail.len());
    tail[..end].parse().ok()
}

fn numeric_defaults(fields: &[(String, String)]) -> BTreeMap<String, f64> {
    fields
        .iter()
        .filter_map(|(key, raw)| parse_number(raw).map(|num| (key.clone(), num)))
        .collect()
}

fn bool_defaults(fields: &[(String, String)]) -> BTreeMap<String, bool> {
    fields
        .iter()
        .filter_map(|(key, raw)| raw.parse::<bool>().ok().map(|flag| (key.clone(), flag)))
        .collect()
}

pub(crate) fn struct_defaults(source: &str, struct_name: &str) -> Defaults {
    let fields: Vec<(String, String)> = impl_block(source, struct_name)
        .lines()
        .filter_map(split_field)
        .collect();
    Defaults {
        nums: numeric_defaults(&fields),
        bools: bool_defaults(&fields),
    }
}
