//! Governance gate .19.3: pretender's effective thresholds must stay within
//! the reviewed bounds in governance/threshold-floor.toml.
//!
//! Effective value = `impl Default` from pretender/src/config.rs merged with
//! any *honored* pretender.toml `[thresholds]` override. The flat
//! `[thresholds]` spelling is what serde honors (flattened onto the app
//! thresholds); the `[thresholds.<table>]` table spelling is ignored by
//! serde, so the gate treats the effective value as the default there —
//! such overrides cannot silently loosen the gate.

// util's gate-run helpers are used only by the findings-subset binary.
#[allow(dead_code)]
#[path = "governance/util.rs"]
mod util;

use std::collections::BTreeMap;
use std::path::Path;

const TABLE_STRUCTS: &[(&str, &str)] = &[
    ("app", "AppThresholds"),
    ("test", "TestThresholds"),
    ("library", "LibraryThresholds"),
    ("script", "ScriptThresholds"),
    ("coupling", "CouplingThresholds"),
    ("unit-test", "DurationThresholds"),
    ("integration-test", "DurationThresholds"),
];

fn struct_name(table: &str) -> &'static str {
    TABLE_STRUCTS
        .iter()
        .find(|(name, _)| *name == table)
        .copied()
        .map(|(_, struct_name)| struct_name)
        .unwrap_or("")
}

fn as_num(value: &toml::Value) -> Option<f64> {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|int| int as f64))
}

/// Direction of a governance bound: ceiling = effective must stay at or
/// below, floor = at or above.
struct Bound {
    name: &'static str,
    allows: fn(f64, f64) -> bool,
}

fn allows_at_most(value: f64, bound: f64) -> bool {
    value <= bound
}

fn allows_at_least(value: f64, bound: f64) -> bool {
    value >= bound
}

const CEILING: Bound = Bound {
    name: "ceiling",
    allows: allows_at_most,
};
const FLOOR: Bound = Bound {
    name: "floor",
    allows: allows_at_least,
};

/// Numeric default values of one thresholds struct, extracted from its
/// `impl Default` block in src/config.rs.
#[derive(Default)]
struct Defaults {
    nums: BTreeMap<String, f64>,
    bools: BTreeMap<String, bool>,
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

fn struct_defaults(source: &str, struct_name: &str) -> Defaults {
    let fields: Vec<(String, String)> = impl_block(source, struct_name)
        .lines()
        .filter_map(split_field)
        .collect();
    Defaults {
        nums: numeric_defaults(&fields),
        bools: bool_defaults(&fields),
    }
}

/// One `[ceilings.<table>]` / `[floors.<table>]` section of the floor file,
/// pre-split into numeric and boolean bounds, with the struct defaults it
/// is checked against.
struct TableCtx<'a> {
    bound: &'static Bound,
    table: String,
    nums: Vec<(String, f64)>,
    flags: Vec<(String, bool)>,
    defaults: &'a Defaults,
}

fn floor_tables(doc: &toml::Value, section: &str) -> Vec<(String, toml::Table)> {
    doc.get(section)
        .and_then(|section| section.as_table())
        .map(|table| {
            table
                .iter()
                .map(|(name, value)| (name.clone(), value.as_table().cloned().unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default()
}

fn numeric_entries(entries: &toml::Table) -> Vec<(String, f64)> {
    entries
        .iter()
        .filter_map(|(key, value)| as_num(value).map(|num| (key.clone(), num)))
        .collect()
}

fn bool_entries(entries: &toml::Table) -> Vec<(String, bool)> {
    entries
        .iter()
        .filter_map(|(key, value)| value.as_bool().map(|flag| (key.clone(), flag)))
        .collect()
}

/// The serde-honored override path for a key: the flat `[thresholds]`
/// spelling for the flattened app table, the subtable spelling otherwise.
fn override_path<'a>(table: &'a str, key: &'a str) -> Vec<&'a str> {
    if table == "app" {
        vec![key]
    } else {
        vec![table, key]
    }
}

struct Gate {
    floor_doc: toml::Value,
    config: toml::Value,
    source: String,
}

impl Gate {
    fn load(root: &Path) -> Self {
        Self {
            floor_doc: toml::from_str(&util::read(root, "governance/threshold-floor.toml"))
                .expect("parse governance/threshold-floor.toml"),
            config: toml::from_str(&util::read(root, "pretender.toml"))
                .expect("parse pretender.toml"),
            source: util::read(root, "pretender/src/config.rs"),
        }
    }

    fn violations(&self) -> Vec<String> {
        let mut out = Vec::new();
        for bound in [&CEILING, &FLOOR] {
            out.extend(self.section_violations(bound));
        }
        out
    }

    fn section_violations(&self, bound: &'static Bound) -> Vec<String> {
        let mut out = Vec::new();
        for (table, entries) in floor_tables(&self.floor_doc, bound.name) {
            let defaults = struct_defaults(&self.source, struct_name(&table));
            let ctx = TableCtx {
                bound,
                table,
                nums: numeric_entries(&entries),
                flags: bool_entries(&entries),
                defaults: &defaults,
            };
            out.extend(ctx.violations_in(self));
        }
        out
    }

    fn override_num(&self, path: &[&str]) -> Option<f64> {
        let mut value = self.config.get("thresholds")?;
        for step in path {
            value = value.get(*step)?;
        }
        as_num(value)
    }

    fn override_flag(&self, path: &[&str]) -> Option<bool> {
        let mut value = self.config.get("thresholds")?;
        for step in path {
            value = value.get(*step)?;
        }
        value.as_bool()
    }
}

impl<'a> TableCtx<'a> {
    fn violations_in(&self, gate: &Gate) -> Vec<String> {
        let mut out = self.num_violations(gate);
        out.extend(self.flag_violations(gate));
        out
    }

    fn num_violations(&self, gate: &Gate) -> Vec<String> {
        self.nums
            .iter()
            .filter_map(|(key, bound)| {
                let path = override_path(&self.table, key);
                let effective = gate
                    .override_num(&path)
                    .or_else(|| self.defaults.nums.get(key).copied());
                let ok = effective.is_some_and(|value| (self.bound.allows)(value, *bound));
                (!ok).then(|| {
                    format!(
                        "{} {}.{}: effective {effective:?} breaks {} {bound}",
                        self.bound.name, self.table, key, self.bound.name
                    )
                })
            })
            .collect()
    }

    fn flag_violations(&self, gate: &Gate) -> Vec<String> {
        self.flags
            .iter()
            .filter_map(|(key, bound)| {
                let path = override_path(&self.table, key);
                let effective = gate
                    .override_flag(&path)
                    .or_else(|| self.defaults.bools.get(key).copied());
                (effective != Some(*bound)).then(|| {
                    format!(
                        "{} {}.{}: effective {effective:?} breaks {} {bound}",
                        self.bound.name, self.table, key, self.bound.name
                    )
                })
            })
            .collect()
    }
}

/// .19.3: every effective threshold must sit inside its reviewed ceiling and
/// floor; the empty violation list proves the merge stayed governed.
#[test]
fn effective_thresholds_respect_governance_bounds() {
    let Some(root) = util::repo_root() else {
        return;
    };
    let gate = Gate::load(&root);
    let violations = gate.violations();
    assert!(
        violations.is_empty(),
        "governance threshold violations:\n{}",
        violations.join("\n")
    );
}
