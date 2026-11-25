//! Governance threshold-floor framework: parses `governance/threshold-floor.toml`
//! and pretender's effective thresholds (`impl Default` merged with any
//! *honored* pretender.toml `[thresholds]` override) and reports every
//! effective value that breaks its reviewed ceiling or floor.
//!
//! The flat `[thresholds]` spelling is what serde honors (flattened onto the
//! app thresholds); the `[thresholds.<table>]` table spelling is ignored by
//! serde, so the gate treats the effective value as the default there —
//! such overrides cannot silently loosen the gate.
//!
//! # Responsibilities
//! Compare the floor file's `[ceilings.*]` / `[floors.*]` tables against the
//! effective values; defaults extraction lives in the `defaults` child
//! module; the repo-root reader is the crate-root `util` module.
//!
//! # Rationale
//! Split out of `governance_thresholds_test.rs` (defaults extraction into
//! `defaults.rs`) so every file stays within the gate's per-file size
//! limit; bodies moved verbatim.

#[path = "defaults.rs"]
mod defaults;

use defaults::{struct_defaults, struct_name, Defaults, TABLE_GROUPS};
use std::path::Path;

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

/// Struct defaults for `table` merged with the flattened sub-struct groups
/// whose `impl Default` blocks carry part of the table's leaf defaults
/// (wave 4 ratchet split the two oversized tables; the TOML spelling is
/// unchanged, so the bounds here are unchanged too).
fn merged_defaults(source: &str, table: &str) -> Defaults {
    let groups = TABLE_GROUPS
        .iter()
        .find(|(name, _)| *name == table)
        .map(|(_, groups)| *groups)
        .unwrap_or(&[]);
    groups.iter().fold(
        struct_defaults(source, struct_name(table)),
        |mut merged, group| {
            let group_defaults = struct_defaults(source, group);
            merged.nums.extend(group_defaults.nums);
            merged.bools.extend(group_defaults.bools);
            merged
        },
    )
}

pub struct Gate {
    floor_doc: toml::Value,
    config: toml::Value,
    source: String,
}

impl Gate {
    pub fn load(root: &Path) -> Self {
        Self {
            floor_doc: toml::from_str(&crate::util::read(root, "governance/threshold-floor.toml"))
                .unwrap_or_else(|e| panic!("parse governance/threshold-floor.toml: {e:?}")),
            config: toml::from_str(&crate::util::read(root, "pretender.toml"))
                .unwrap_or_else(|e| panic!("parse pretender.toml: {e:?}")),
            // The defaults live across the config module files; a single
            // concatenated source keeps the `impl Default for <Struct>`
            // search working over the whole tree.
            source: format!(
                "{}{}{}",
                crate::util::read(root, "pretender/src/config.rs"),
                crate::util::read(root, "pretender/src/config/thresholds.rs"),
                crate::util::read(root, "pretender/src/config/sections.rs"),
            ),
        }
    }

    pub fn violations(&self) -> Vec<String> {
        [&CEILING, &FLOOR]
            .iter()
            .flat_map(|bound| self.section_violations(bound))
            .collect()
    }

    fn section_violations(&self, bound: &'static Bound) -> Vec<String> {
        let table_out: Vec<Vec<String>> = floor_tables(&self.floor_doc, bound.name)
            .iter()
            .map(|(table, entries)| {
                let defaults = merged_defaults(&self.source, table);
                let ctx = TableCtx {
                    bound,
                    table: table.clone(),
                    nums: numeric_entries(entries),
                    flags: bool_entries(entries),
                    defaults: &defaults,
                };
                ctx.violations_in(self)
            })
            .collect();
        table_out.into_iter().flatten().collect()
    }

    /// The serde-honored override value for a `[thresholds]` path.
    fn override_value<'v>(&'v self, path: &[&str]) -> Option<&'v toml::Value> {
        let mut value = self.config.get("thresholds")?;
        for step in path {
            value = value.get(*step)?;
        }
        Some(value)
    }

    fn override_num(&self, path: &[&str]) -> Option<f64> {
        self.override_value(path).and_then(as_num)
    }

    fn override_flag(&self, path: &[&str]) -> Option<bool> {
        self.override_value(path).and_then(|value| value.as_bool())
    }
}

impl<'a> TableCtx<'a> {
    fn violations_in(&self, gate: &Gate) -> Vec<String> {
        self.num_violations(gate)
            .into_iter()
            .chain(self.flag_violations(gate))
            .collect()
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
