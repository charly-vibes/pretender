//! Query-engine core: the compiled tree-sitter query, its capture-slot
//! index, and the weight-override bundle adapters supply.
//!
//! # Responsibilities
//! Compile adapter query sources, resolve every capture index into
//! [`CaptureSlots`] (grouped into required function slots, optional
//! call/import slots, and optional type slots), and expose
//! [`WeightMaps`] as the single bundle of adapter weight overrides.
//! The recursive body walk lives in the sibling `walk` module.
//!
//! # Rationale
//! Split from the former monolithic `engine.rs` (pretender-5zo.3,
//! ratcheted in pretender-5zo.15.3): query construction stays here
//! while node traversal moved to `walk.rs`, and the former 15-field
//! slot struct is grouped into role sub-structs so no struct exceeds
//! the field cap.

use anyhow::{Context, Result};
use std::collections::BTreeMap;

use crate::metric_application::BranchCaptureSpec;
use crate::model::Language;
use crate::plugin::{BranchWeights, NestingConfig};

mod clause;
mod walk;

pub(crate) use walk::{build_block, count_captured_nodes, has_test_marker, node_span, WalkCtx};

pub struct QueryEngine {
    pub(crate) language: tree_sitter::Language,
    pub(crate) lang_kind: Language,
    pub(crate) query: tree_sitter::Query,
    pub(crate) slots: CaptureSlots,
    pub(crate) call_weights: BTreeMap<String, f64>,
    /// D4: adapter opted into branch-body nesting (Clojure).
    pub(crate) wrap_branch_bodies: bool,
    /// The compiled query's source, kept so match collection can
    /// re-validate `#match?`/`#eq?`/`#any-of?` predicates (krou: the
    /// binding exposes native text predicates only privately).
    pub(crate) query_source: Box<str>,
}

/// Every capture index one compiled query provides, grouped by role:
/// the four required function captures, the weighted branch captures,
/// the assertion capture ids, and the optional call/import/type
/// captures (D2).
pub(crate) struct CaptureSlots {
    pub(crate) fn_def_idx: u32,
    pub(crate) fn_name_idx: u32,
    pub(crate) fn_params_idx: u32,
    pub(crate) fn_body_idx: u32,
    pub(crate) branch_captures: Vec<(u32, BranchCaptureSpec)>,
    pub(crate) assert_capture_indices: Vec<u32>,
    pub(crate) optionals: OptionalCaptureSlots,
    pub(crate) types: TypeCaptureSlots,
}

/// Optional call/assign/import captures (D2): absent in languages
/// whose grammar has no corresponding construct.
pub(crate) struct OptionalCaptureSlots {
    pub(crate) call_idx: Option<u32>,
    pub(crate) call_callee_idx: Option<u32>,
    pub(crate) assign_idx: Option<u32>,
    pub(crate) import_idx: Option<u32>,
    pub(crate) consumed_idx: Option<u32>,
    pub(crate) clause_dispatch_idx: Option<u32>,
}

/// Optional type captures (D2): their absence leaves type_definitions
/// empty instead of failing the query, mirroring how @import is handled.
pub(crate) struct TypeCaptureSlots {
    pub(crate) type_def_idx: Option<u32>,
    pub(crate) type_name_idx: Option<u32>,
    pub(crate) type_field_idx: Option<u32>,
    pub(crate) type_field_name_idx: Option<u32>,
    pub(crate) type_field_type_idx: Option<u32>,
}

/// Compile `query_source` for `language`.
pub(crate) fn compile_query(
    language: &tree_sitter::Language,
    query_source: &str,
) -> Result<tree_sitter::Query> {
    tree_sitter::Query::new(language, query_source)
        .map_err(|e| anyhow::anyhow!("failed to compile query: {e}"))
}

fn require_capture_idx(query: &tree_sitter::Query, name: &str) -> Result<u32> {
    query
        .capture_index_for_name(name)
        .with_context(|| format!("query missing @{name} capture"))
}

fn assert_capture_indices(query: &tree_sitter::Query) -> Vec<u32> {
    query
        .capture_names()
        .iter()
        .enumerate()
        .filter_map(|(idx, name)| name.starts_with("assert.").then_some(idx as u32))
        .collect()
}

fn optional_capture_slots(query: &tree_sitter::Query) -> OptionalCaptureSlots {
    OptionalCaptureSlots {
        call_idx: query.capture_index_for_name("call"),
        call_callee_idx: query.capture_index_for_name("call.callee"),
        assign_idx: query.capture_index_for_name("assign"),
        import_idx: query.capture_index_for_name("import"),
        consumed_idx: query.capture_index_for_name("consumed"),
        clause_dispatch_idx: query.capture_index_for_name("clause.dispatch"),
    }
}

fn type_capture_slots(query: &tree_sitter::Query) -> TypeCaptureSlots {
    TypeCaptureSlots {
        type_def_idx: query.capture_index_for_name("type.definition"),
        type_name_idx: query.capture_index_for_name("type.name"),
        type_field_idx: query.capture_index_for_name("type.field"),
        type_field_name_idx: query.capture_index_for_name("type.field.name"),
        type_field_type_idx: query.capture_index_for_name("type.field.type"),
    }
}

/// Resolve every capture index of `query`, carrying `branch_captures`
/// (already weight-resolved by the metric layer) into the slots.
pub(crate) fn index_capture_slots(
    query: &tree_sitter::Query,
    branch_captures: Vec<(u32, BranchCaptureSpec)>,
) -> Result<CaptureSlots> {
    Ok(CaptureSlots {
        fn_def_idx: require_capture_idx(query, "function.definition")?,
        fn_name_idx: require_capture_idx(query, "function.name")?,
        fn_params_idx: require_capture_idx(query, "function.parameters")?,
        fn_body_idx: require_capture_idx(query, "function.body")?,
        branch_captures,
        assert_capture_indices: assert_capture_indices(query),
        optionals: optional_capture_slots(query),
        types: type_capture_slots(query),
    })
}

/// Weight overrides supplied by a language adapter, passed to
/// [`QueryEngine::new_with_branch_weights`] as one bundle.
pub struct WeightMaps<'a> {
    /// Per-branch-name cyclomatic/cognitive weight overrides.
    pub branch: &'a BTreeMap<String, BranchWeights>,
    /// Per-callee smell-weight overrides.
    pub call: &'a BTreeMap<String, f64>,
    /// Nesting-walk configuration (D4 body wrapping opt-in).
    pub nesting: &'a NestingConfig,
}
