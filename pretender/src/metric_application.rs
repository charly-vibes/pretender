//! Metric application for [`QueryEngine`]: turns captured function
//! definitions into metric-bearing `CodeUnit`s.
//!
//! Purpose: resolve adapter-supplied metric weights, apply them while
//! walking function bodies, and assemble each unit's name, kind,
//! parameters, assertions, and weighted block tree — the aggregate
//! input the metrics pipeline consumes.
//!
//! # Responsibilities
//! Weight resolution (`WeightMaps` overrides into per-branch cyclomatic/
//! cognitive specs) and per-unit assembly (`build_code_unit`: name, kind,
//! parameters, assertions, and the weighted block tree the metrics
//! pipeline consumes). The recursive body walk itself lives in
//! `engine.rs` as `QueryEngine`'s walk plumbing.
//!
//! # Rationale
//! Split out of `engine.rs` so the module boundary follows the
//! responsibility; `new_with_branch_weights` and `extract_params` are
//! decomposed into stage helpers so every function fits the gate's
//! limits, with bodies moved verbatim to keep unit output
//! byte-identical.

use std::collections::{BTreeMap, HashMap};

use anyhow::Result;

use crate::engine::{
    WeightMaps, WalkCtx, build_block, compile_query, count_captured_nodes, has_test_marker,
    index_capture_slots, node_span, QueryEngine,
};
use crate::model::{Block, BranchKind, CodeUnit, Language, Parameter, UnitKind};
use crate::plugin::BranchWeights;

#[derive(Clone, Copy)]
pub(crate) struct BranchCaptureSpec {
    pub(crate) kind: BranchKind,
    pub(crate) cyclomatic_weight: u32,
    pub(crate) cognitive_weight: u32,
}

impl QueryEngine {
    #[allow(dead_code)]
    pub fn new(
        language: tree_sitter::Language,
        lang_kind: Language,
        query_source: &str,
    ) -> Result<Self> {
        let empty_branches = BTreeMap::new();
        let empty_calls = BTreeMap::new();
        Self::new_with_branch_weights(
            language,
            lang_kind,
            query_source,
            &WeightMaps {
                branch: &empty_branches,
                call: &empty_calls,
            },
        )
    }

    pub fn new_with_branch_weights(
        language: tree_sitter::Language,
        lang_kind: Language,
        query_source: &str,
        weights: &WeightMaps<'_>,
    ) -> Result<Self> {
        let query = compile_query(&language, query_source)?;
        let branch_captures = resolve_branch_captures(&query, weights);
        let slots = index_capture_slots(&query, branch_captures)?;
        Ok(Self {
            language,
            lang_kind,
            query,
            slots,
            call_weights: weights.call.clone(),
        })
    }

    pub(crate) fn branch_capture_for_index(&self, idx: u32) -> Option<BranchCaptureSpec> {
        self.slots
            .branch_captures
            .iter()
            .find(|(capture_idx, _)| *capture_idx == idx)
            .map(|(_, spec)| *spec)
    }

    pub(crate) fn build_code_unit(
        &self,
        fc: &FunctionCapture,
        source: &[u8],
        captures: &CaptureMap,
    ) -> Result<CodeUnit, String> {
        let def_node = fc.def_node.ok_or("missing definition node")?;
        let name_node = fc.name_node.ok_or("missing name node")?;
        let body_node = fc.body_node.ok_or("missing body node")?;

        let name = unit_name(name_node, source)?;
        let parameters = unit_parameters(fc, source);
        let body = self.unit_body(body_node, captures);
        let kind = determine_unit_kind(def_node, &name);
        let is_exported = !name.starts_with('_');

        Ok(CodeUnit {
            name,
            kind,
            span: node_span(def_node),
            parameters,
            body,
            is_exported,
            assertions: count_captured_nodes(body_node, &captures.assertions),
            parent_class: None,
            has_test_attribute: has_test_marker(def_node, source),
        })
    }

    fn unit_body(&self, body_node: tree_sitter::Node, captures: &CaptureMap) -> Block {
        let walk_ctx = WalkCtx {
            captures,
            call_weights: &self.call_weights,
            nesting: 0,
        };
        build_block(&walk_ctx, body_node)
    }
}

/// Resolve adapter weight overrides into per-capture-index branch specs,
/// skipping branch kinds the query does not capture.
fn resolve_branch_captures(
    query: &tree_sitter::Query,
    weights: &WeightMaps<'_>,
) -> Vec<(u32, BranchCaptureSpec)> {
    let branch_mapping = [
        ("branch.if", BranchKind::If),
        ("branch.elif", BranchKind::ElseIf),
        ("branch.switch_case", BranchKind::SwitchCase),
        ("branch.loop", BranchKind::Loop),
        ("branch.catch", BranchKind::Catch),
        ("branch.ternary", BranchKind::Ternary),
        ("branch.logical.and", BranchKind::LogicalAnd),
        ("branch.logical.or", BranchKind::LogicalOr),
    ];
    branch_mapping
        .iter()
        .filter_map(|(name, kind)| {
            query
                .capture_index_for_name(name)
                .map(|idx| (idx, branch_spec(weights, name, kind)))
        })
        .collect()
}

/// One branch capture spec: the adapter's cyclomatic/cognitive weight
/// override for `@name`, or the neutral default of 1/1.
fn branch_spec(weights: &WeightMaps<'_>, name: &str, kind: &BranchKind) -> BranchCaptureSpec {
    let capture_name = format!("@{name}");
    let defaults = BranchWeights {
        cyclomatic: 1,
        cognitive: 1,
    };
    let w = weights.branch.get(&capture_name).copied().unwrap_or(defaults);
    BranchCaptureSpec {
        kind: *kind,
        cyclomatic_weight: w.cyclomatic,
        cognitive_weight: w.cognitive,
    }
}

#[derive(Default)]
pub(crate) struct CaptureMap {
    pub(crate) branches: HashMap<usize, BranchCaptureSpec>,
    pub(crate) calls: HashMap<usize, String>,
    pub(crate) assignments: std::collections::HashSet<usize>,
    pub(crate) assertions: std::collections::HashSet<usize>,
}

#[derive(Default)]
pub(crate) struct FunctionCapture<'a> {
    pub(crate) def_node: Option<tree_sitter::Node<'a>>,
    pub(crate) name_node: Option<tree_sitter::Node<'a>>,
    pub(crate) params_node: Option<tree_sitter::Node<'a>>,
    pub(crate) body_node: Option<tree_sitter::Node<'a>>,
}

fn determine_unit_kind(node: tree_sitter::Node, name: &str) -> UnitKind {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "class_definition" => {
                if name == "__init__" || name == "__new__" {
                    return UnitKind::Ctor;
                }
                return UnitKind::Method;
            }
            "function_definition" => return UnitKind::Function,
            "module" => break,
            _ => {
                current = parent.parent();
            }
        }
    }
    UnitKind::Function
}

fn extract_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<Parameter> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if let Some(name) = param_name(&child, source) {
            params.push(param_entry(&child, name, source));
        }
    }
    params
}

/// The parameter name a grammar-specific parameter node carries, if any.
fn param_name(child: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    match child.kind() {
        "identifier" => child.utf8_text(source).ok().map(str::to_string),
        "typed_parameter" | "default_parameter" | "typed_default_parameter" => child
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .map(str::to_string),
        "parameter" => pattern_param_name(child, source),
        // Rust `self` parameters (`fn f(&self)` / `fn f(&mut self)`).
        "self_parameter" | "self" => Some("self".to_string()),
        "list_splat_pattern" | "dictionary_splat_pattern" => splat_param_name(*child, source),
        _ => None,
    }
}

/// Python/Go-style parameters expose a `name` field; Rust
/// (tree-sitter-rust) exposes a `pattern` field instead.
fn pattern_param_name(child: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    child
        .child_by_field_name("name")
        .or_else(|| child.child_by_field_name("pattern"))
        .and_then(|pattern| {
            if pattern.kind() == "self" {
                return Some("self".to_string());
            }
            pattern_ident(&pattern, source)
                .or_else(|| pattern.utf8_text(source).ok().map(str::to_string))
        })
}

/// First identifier child of a pattern node, if any.
fn pattern_ident(pattern: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut c = pattern.walk();
    for n in pattern.children(&mut c) {
        if n.kind() == "identifier" {
            return n.utf8_text(source).ok().map(str::to_string);
        }
    }
    None
}

/// Splat parameters keep their `*`/`**` prefix before the identifier.
fn splat_param_name(child: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let prefix = if child.kind() == "list_splat_pattern" {
        "*"
    } else {
        "**"
    };
    let mut c = child.walk();
    let ident = child
        .children(&mut c)
        .find(|n| n.kind() == "identifier")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| format!("{prefix}{s}"));
    ident
}

fn param_entry(child: &tree_sitter::Node, name: String, source: &[u8]) -> Parameter {
    Parameter {
        span: node_span(*child),
        name,
        type_name: child
            .child_by_field_name("type")
            .and_then(|n| n.utf8_text(source).ok())
            .map(str::to_string),
    }
}

fn unit_name(name_node: tree_sitter::Node, source: &[u8]) -> Result<String, String> {
    Ok(name_node
        .utf8_text(source)
        .map_err(|_| "name contains invalid UTF-8")?
        .to_string())
}

fn unit_parameters(fc: &FunctionCapture, source: &[u8]) -> Vec<Parameter> {
    fc.params_node
        .map(|p| extract_params(p, source))
        .unwrap_or_default()
}
