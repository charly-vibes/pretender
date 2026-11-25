//! Per-match capture dispatch: one [`PendingMatch`] accumulates a single
//! query match's non-function captures in capture order.
//!
//! # Responsibilities
//! Route each capture to the metric maps (branch/assertion/assignment),
//! the pending call site, the import list, or one of the five type-capture
//! slots — in the original if-chain order, so findings stay byte-identical.
//!
//! # Rationale
//! Split from the parse pipeline (pretender-5zo.15.12); bodies moved
//! verbatim, with the seven-arm `dispatch_type` if-chain split into
//! order-preserving helpers to stay within the complexity limits.
use super::CollectedCaptures;
use crate::engine::QueryEngine;
use crate::metric_application::CaptureMap;
use crate::model::Import;

/// Per-match slots: a pending call site plus five optional type captures.
pub(super) struct PendingMatch<'a, 'e> {
    engine: &'e QueryEngine,
    source: &'a [u8],
    pub(super) pending_call: Option<(usize, Option<String>)>,
    pub(super) type_def_node: Option<tree_sitter::Node<'a>>,
    pub(super) type_name_node: Option<tree_sitter::Node<'a>>,
    pub(super) type_field_node: Option<tree_sitter::Node<'a>>,
    pub(super) type_field_name_node: Option<tree_sitter::Node<'a>>,
    pub(super) type_field_type_node: Option<tree_sitter::Node<'a>>,
}

impl<'a, 'e> PendingMatch<'a, 'e> {
    pub(super) fn new(engine: &'e QueryEngine, source: &'a [u8]) -> Self {
        Self {
            engine,
            source,
            pending_call: None,
            type_def_node: None,
            type_name_node: None,
            type_field_node: None,
            type_field_name_node: None,
            type_field_type_node: None,
        }
    }

    /// Absorb one capture, trying metric, call, and type dispatch in the
    /// original order. Consuming so the driver needs no mutable binding.
    pub(super) fn dispatch(
        mut self,
        capture: &tree_sitter::QueryCapture<'a>,
        out: &mut super::CollectedCaptures<'a>,
    ) -> Self {
        if self.dispatch_metric(&mut out.map, capture) {
            return self;
        }
        if self.dispatch_call(capture) {
            return self;
        }
        self.dispatch_type(out, capture);
        self
    }

    /// Branch/assertion/assignment captures feed the metric maps.
    fn dispatch_metric(
        &mut self,
        map: &mut CaptureMap,
        capture: &tree_sitter::QueryCapture<'a>,
    ) -> bool {
        let engine = self.engine;
        let ix = capture.index;
        let asserts = &engine.slots.assert_capture_indices;
        if let Some(spec) = engine.branch_capture_for_index(ix) {
            map.branches.insert(capture.node.id(), spec);
        } else if asserts.contains(&ix) {
            map.assertions.insert(capture.node.id());
        } else if Some(ix) == engine.slots.optionals.assign_idx {
            map.assignments.insert(capture.node.id());
        } else {
            return Self::dispatch_neutral_capture(map, engine, ix, capture);
        }
        true
    }

    /// Neutral metric captures: forms claimed without emitting a model
    /// node, and per-clause dispatch heads feeding clause counting.
    fn dispatch_neutral_capture(
        map: &mut CaptureMap,
        engine: &QueryEngine,
        ix: u32,
        capture: &tree_sitter::QueryCapture<'a>,
    ) -> bool {
        if Some(ix) == engine.slots.optionals.consumed_idx {
            map.consumed.insert(capture.node.id());
        } else if Some(ix) == engine.slots.optionals.clause_dispatch_idx {
            map.dispatch_heads.insert(capture.node.id());
        } else {
            return false;
        }
        true
    }

    /// Call and call.callee captures accumulate into the pending call site.
    fn dispatch_call(&mut self, capture: &tree_sitter::QueryCapture<'a>) -> bool {
        if Some(capture.index) == self.engine.slots.optionals.call_idx {
            let id = capture.node.id();
            match &mut self.pending_call {
                Some((existing_id, _)) if *existing_id == id => {}
                _ => self.pending_call = Some((id, None)),
            }
        } else if Some(capture.index) == self.engine.slots.optionals.call_callee_idx {
            let callee = capture.node.utf8_text(self.source).ok().map(str::to_string);
            if let Some((_, slot)) = self.pending_call.as_mut() {
                *slot = callee;
            }
        } else {
            return false;
        }
        true
    }

    /// Import captures go straight into `out`; type captures fill slots.
    fn dispatch_type(
        &mut self,
        out: &mut CollectedCaptures<'a>,
        capture: &tree_sitter::QueryCapture<'a>,
    ) -> bool {
        if Some(capture.index) == self.engine.slots.optionals.import_idx {
            push_import(out, capture, self.source);
            return true;
        }
        self.dispatch_type_def(capture) || self.dispatch_type_field(capture)
    }

    /// `@type.definition` / `@type.name` slot captures, in chain order.
    fn dispatch_type_def(&mut self, capture: &tree_sitter::QueryCapture<'a>) -> bool {
        if Some(capture.index) == self.engine.slots.types.type_def_idx {
            self.type_def_node = Some(capture.node);
            return true;
        }
        if Some(capture.index) == self.engine.slots.types.type_name_idx {
            self.type_name_node = Some(capture.node);
            return true;
        }
        false
    }

    /// `@type.field` / `@type.field.name` / `@type.field.type` slot
    /// captures, in chain order.
    fn dispatch_type_field(&mut self, capture: &tree_sitter::QueryCapture<'a>) -> bool {
        if Some(capture.index) == self.engine.slots.types.type_field_idx {
            self.type_field_node = Some(capture.node);
            return true;
        }
        if Some(capture.index) == self.engine.slots.types.type_field_name_idx {
            self.type_field_name_node = Some(capture.node);
            return true;
        }
        if Some(capture.index) == self.engine.slots.types.type_field_type_idx {
            self.type_field_type_node = Some(capture.node);
            return true;
        }
        false
    }
}

/// Commit an import capture's text as an `Import`.
fn push_import(
    out: &mut CollectedCaptures<'_>,
    capture: &tree_sitter::QueryCapture<'_>,
    source: &[u8],
) {
    if let Ok(text) = capture.node.utf8_text(source) {
        out.imports.push(Import {
            module: text.to_string(),
            name: None,
            alias: None,
            span: crate::engine::node_span(capture.node),
        });
    }
}
