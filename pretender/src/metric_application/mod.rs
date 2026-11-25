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
//! cognitive specs) and per-unit assembly (`build_code_unit`). The
//! recursive body walk lives in `engine.rs` as `QueryEngine`'s walk
//! plumbing; the parameter-extraction helpers in `params`.
//!
//! # Rationale
//! Split out of `engine.rs` so the module boundary follows the
//! responsibility; helpers are decomposed into stage functions so every
//! function fits the gate's limits, with bodies moved verbatim to keep
//! unit output byte-identical.

use std::collections::{BTreeMap, HashMap};

use anyhow::Result;

use crate::engine::{
    build_block, compile_query, count_captured_nodes, has_test_marker, index_capture_slots,
    node_span, QueryEngine, WalkCtx, WeightMaps,
};
use crate::model::{Block, BranchKind, CodeUnit, Language, UnitKind};
use crate::plugin::{BranchWeights, NestingConfig};

/// A tree-sitter node borrowed from the parsed source.
type TsNode<'a> = tree_sitter::Node<'a>;

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
        let default_nesting = NestingConfig::default();
        Self::new_with_branch_weights(
            language,
            lang_kind,
            query_source,
            &WeightMaps {
                branch: &empty_branches,
                call: &empty_calls,
                nesting: &default_nesting,
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
            wrap_branch_bodies: weights.nesting.wrap_branch_bodies,
            query_source: query_source.into(),
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
        let (def_node, name_node, body_node) = required_capture_nodes(fc)?;
        let (name, kind, is_exported) = unit_identity(def_node, name_node, source)?;
        Ok(CodeUnit {
            name,
            kind,
            span: node_span(fc.span_node.unwrap_or(def_node)),
            parameters: unit_parameters(fc, source),
            body: self.unit_body(body_node, captures),
            parent_class: None,
            has_test_attribute: has_test_marker(def_node, source),
            is_exported,
            assertions: count_captured_nodes(body_node, &captures.assertions),
        })
    }

    fn unit_body(&self, body_node: tree_sitter::Node, captures: &CaptureMap) -> Block {
        let walk_ctx = WalkCtx {
            captures,
            call_weights: &self.call_weights,
            wrap_branch_bodies: self.wrap_branch_bodies,
            nesting: 0,
        };
        build_block(&walk_ctx, body_node)
    }
}

/// The definition, name, and body nodes a code unit is built from.
fn required_capture_nodes<'a>(
    fc: &FunctionCapture<'a>,
) -> Result<(TsNode<'a>, TsNode<'a>, TsNode<'a>), String> {
    Ok((
        fc.def_node.ok_or("missing definition node")?,
        fc.name_node.ok_or("missing name node")?,
        fc.body_node.ok_or("missing body node")?,
    ))
}

/// Resolve a unit's name, kind, and export flag from its captures.
fn unit_identity(
    def_node: TsNode<'_>,
    name_node: TsNode<'_>,
    source: &[u8],
) -> Result<(String, UnitKind, bool), String> {
    let name = unit_name(name_node, source)?;
    let is_exported = !name.starts_with('_');
    let kind = determine_unit_kind(def_node, &name, source);
    Ok((name, kind, is_exported))
}

/// Query capture names mapped to the branch kinds they represent.
const BRANCH_MAPPING: [(&str, BranchKind); 12] = [
    ("branch.if", BranchKind::If),
    ("branch.elif", BranchKind::ElseIf),
    ("branch.switch_case", BranchKind::SwitchCase),
    ("branch.loop", BranchKind::Loop),
    ("branch.catch", BranchKind::Catch),
    ("branch.ternary", BranchKind::Ternary),
    ("branch.logical.and", BranchKind::LogicalAnd),
    ("branch.logical.or", BranchKind::LogicalOr),
    ("branch.clause", BranchKind::CondClause),
    ("branch.recur", BranchKind::Recur),
    ("branch.match_case", BranchKind::MatchCase),
    (
        "branch.comprehension_filter",
        BranchKind::ComprehensionFilter,
    ),
];

/// Resolve adapter weight overrides into per-capture-index branch specs,
/// skipping branch kinds the query does not capture.
fn resolve_branch_captures(
    query: &tree_sitter::Query,
    weights: &WeightMaps<'_>,
) -> Vec<(u32, BranchCaptureSpec)> {
    BRANCH_MAPPING
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
    let w = weights
        .branch
        .get(&capture_name)
        .copied()
        .unwrap_or(defaults);
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
    /// Forms claimed by the adapter that emit no model node (`@consumed`):
    /// binding/sequence/error-scaffold forms which must not fall through
    /// to the call capture (D3 capture precedence).
    pub(crate) consumed: std::collections::HashSet<usize>,
    /// Per-clause dispatch-head children (`@clause.dispatch`): their
    /// count sets how many leading children a clause form skips before
    /// test/result pairs start.
    pub(crate) dispatch_heads: std::collections::HashSet<usize>,
}

#[derive(Default, Clone)]
pub(crate) struct FunctionCapture<'a> {
    pub(crate) def_node: Option<tree_sitter::Node<'a>>,
    pub(crate) name_node: Option<tree_sitter::Node<'a>>,
    pub(crate) params_node: Option<tree_sitter::Node<'a>>,
    pub(crate) body_node: Option<tree_sitter::Node<'a>>,
    // Span scope; defaults to `def_node` (split arities use the clause).
    pub(crate) span_node: Option<tree_sitter::Node<'a>>,
}

/// The unit kind for a captured definition: `Test` for a Clojure
/// `(deftest …)` form, otherwise decided by the enclosing scopes.
fn determine_unit_kind(node: tree_sitter::Node, name: &str, source: &[u8]) -> UnitKind {
    if clojure_deftest_form(node, source) {
        return UnitKind::Test;
    }
    kind_from_scopes(node, name, source)
}

/// The kind implied by the enclosing scopes: Python class/function
/// parents, or the Clojure enclosing `(head …)` list.
fn kind_from_scopes(node: tree_sitter::Node, name: &str, source: &[u8]) -> UnitKind {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "class_definition" => return class_kind(name),
            "function_definition" => return UnitKind::Function,
            "list_lit" => {
                // Clojure: the enclosing head symbol decides the kind.
                if let Some(kind) = clojure_scope_kind(parent, source) {
                    return kind;
                }
                current = parent.parent();
            }
            "module" => break,
            _ => current = parent.parent(),
        }
    }
    UnitKind::Function
}

/// True when the node is a Clojure `(deftest …)` registration form.
fn clojure_deftest_form(node: tree_sitter::Node, source: &[u8]) -> bool {
    node.kind() == "list_lit"
        && node.named_child(0).is_some_and(|first| {
            first.kind() == "sym_lit" && first.utf8_text(source).is_ok_and(|text| text == "deftest")
        })
}

/// The Python class-member kind: constructors get `Ctor`, everything
/// else `Method`.
fn class_kind(name: &str) -> UnitKind {
    if name == "__init__" || name == "__new__" {
        return UnitKind::Ctor;
    }
    UnitKind::Method
}

/// The kind implied by a Clojure enclosing `(head …)` scope, if the head
/// symbol decides one.
fn clojure_scope_kind(parent: tree_sitter::Node, source: &[u8]) -> Option<UnitKind> {
    match parent
        .named_child(0)
        .and_then(|head| head.utf8_text(source).ok())
    {
        Some("defrecord" | "deftype" | "defprotocol") => Some(UnitKind::Method),
        Some("defn" | "defn-" | "deftest") => Some(UnitKind::Function),
        _ => None,
    }
}

fn unit_name(name_node: tree_sitter::Node, source: &[u8]) -> Result<String, String> {
    Ok(name_node
        .utf8_text(source)
        .map_err(|_| "name contains invalid UTF-8")?
        .to_string())
}

mod params;

use params::unit_parameters;
