//! Parse pipeline for [`QueryEngine`]: turns tree-sitter query matches over
//! one source file into a [`Module`] — function captures become `CodeUnit`s,
//! metric captures feed the branch/call/assignment maps, type captures feed
//! type-definition building. Bodies moved verbatim from `engine.rs`, or
//! split along order-preserving seams, so findings stay byte-identical.
//!
//! Per-match dispatch lives in `super::parse::pending`; match absorption and
//! module assembly in `super::parse::assemble`.

use std::path::Path;

use anyhow::{Context, Result};
use assemble::{absorb_type_defs, absorb_type_fields, build_type_captures, parse_error_diagnostic};
use streaming_iterator::StreamingIterator;

use crate::engine::{node_span, QueryEngine};
use crate::line_counts::LineCounts;
use crate::metric_application::{CaptureMap, FunctionCapture};
use crate::model::{CodeUnit, Diagnostic, DiagnosticSeverity, Import, Module};
use crate::type_definitions::{TypeDefCapture, TypeFieldCapture};

mod assemble;
mod extract;
mod pending;
mod predicates;

use predicates::PredicateChecks;

impl QueryEngine {
    /// Parse `source` into a [`Module`] plus any parse diagnostics.
    pub fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        let tree = self.parse_tree(source)?;
        let root = tree.root_node();
        let lines = LineCounts::collect(root, source);
        let base = assemble::base_module(path, &self.lang_kind, lines);
        if root.has_error() {
            return Ok((base, vec![parse_error_diagnostic(path, root)]));
        }
        Ok(self.parsed_module(source, root, base))
    }

    /// The full module for a clean tree: captures → units + imports + types.
    fn parsed_module(
        &self,
        source: &str,
        root: tree_sitter::Node<'_>,
        base: Module,
    ) -> (Module, Vec<Diagnostic>) {
        let source_bytes = source.as_bytes();
        let captures =
            self.collect_query_captures(root, source_bytes, CollectedCaptures::default());
        let (units, diagnostics) = self.build_units(&captures, source_bytes);
        let (type_definitions, captures) =
            build_type_captures(captures, source_bytes, &self.lang_kind);
        let module = assemble::finished_module(base, units, captures.imports, type_definitions);
        (module, diagnostics)
    }

    fn parse_tree(&self, source: &str) -> Result<tree_sitter::Tree> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.language)
            .context("failed to set language")?;
        parser
            .parse(source.as_bytes(), None)
            .context("tree-sitter returned no tree")
    }

    /// Bundle the captures of one `@function.definition` match.
    fn function_capture<'a>(&self, m: &tree_sitter::QueryMatch<'_, 'a>) -> FunctionCapture<'a> {
        m.captures
            .iter()
            .fold(FunctionCapture::default(), |mut fc, capture| {
                if capture.index == self.slots.fn_def_idx {
                    fc.def_node = Some(capture.node);
                } else if capture.index == self.slots.fn_name_idx {
                    fc.name_node = Some(capture.node);
                } else if capture.index == self.slots.fn_params_idx {
                    fc.params_node = Some(capture.node);
                } else if capture.index == self.slots.fn_body_idx {
                    fc.body_node = Some(capture.node);
                }
                fc
            })
        // Zero derives from FunctionCapture::default; every real
        // pattern index is assigned right after by the caller.
    }

    /// One pass over the query matches, in match order.
    fn collect_query_captures<'a>(
        &self,
        root: tree_sitter::Node<'a>,
        source_bytes: &'a [u8],
        mut out: CollectedCaptures<'a>,
    ) -> CollectedCaptures<'a> {
        let mut query_cursor = tree_sitter::QueryCursor::new();
        let mut query_matches = query_cursor.matches(&self.query, root, source_bytes);
        let checks = PredicateChecks::extract(&self.query, &self.query_source);
        let scan = MatchScan {
            checks: &checks,
            source_bytes,
        };
        while let Some(m) = query_matches.next() {
            self.absorb_match(m, &scan, &mut out);
        }
        out
    }

    /// Absorb one query match: re-validate its predicates, then route
    /// function vs non-function captures.
    fn absorb_match<'a>(
        &self,
        m: &tree_sitter::QueryMatch<'_, 'a>,
        scan: &MatchScan<'a, '_>,
        out: &mut CollectedCaptures<'a>,
    ) {
        if !scan.checks.pass(m, scan.source_bytes) {
            return;
        }
        if m.captures.iter().any(|c| c.index == self.slots.fn_def_idx) {
            let mut fc = self.function_capture(m);
            fc.pattern_index = m.pattern_index;
            out.functions.push(fc);
        } else {
            self.collect_non_function_match(m, scan.source_bytes, out);
        }
    }

    /// Collect a non-function match: metric captures go straight into `out`.
    fn collect_non_function_match<'a>(
        &self,
        m: &tree_sitter::QueryMatch<'_, 'a>,
        source_bytes: &'a [u8],
        out: &mut CollectedCaptures<'a>,
    ) {
        let pending = m.captures.iter().fold(
            pending::PendingMatch::new(self, source_bytes),
            |pending, capture| pending.dispatch(capture, out),
        );
        pending_flush(out, pending);
    }

    /// Build one `CodeUnit` per function capture, pairing failures with diagnostics.
    fn build_units<'a>(
        &self,
        captures: &CollectedCaptures<'a>,
        source_bytes: &[u8],
    ) -> (Vec<CodeUnit>, Vec<Diagnostic>) {
        captures.functions.iter().fold(
            (Vec::new(), Vec::new()),
            |(mut units, mut diagnostics), fc| {
                match self.build_code_unit(fc, source_bytes, &captures.map) {
                    Ok(unit) => units.push(unit),
                    Err(e) => diagnostics.push(Diagnostic {
                        message: format!("Failed to build CodeUnit: {e}"),
                        span: fc.def_node.map(node_span),
                        severity: DiagnosticSeverity::Error,
                    }),
                }
                (units, diagnostics)
            },
        )
    }
}

/// Flush a finished match: commit the pending call site and type captures.
fn pending_flush<'a>(out: &mut CollectedCaptures<'a>, mut pending: pending::PendingMatch<'a, '_>) {
    if let Some((id, callee)) = pending.pending_call.take() {
        out.map.calls.insert(id, callee.unwrap_or_default());
    }
    absorb_type_defs(out, &pending);
    absorb_type_fields(out, &pending);
}

/// Accumulators filled by one pass over the query matches, in match order.
#[derive(Default)]
pub(super) struct CollectedCaptures<'a> {
    functions: Vec<FunctionCapture<'a>>,
    pub(super) map: CaptureMap,
    pub(super) imports: Vec<Import>,
    pub(super) type_defs: Vec<TypeDefCapture<'a>>,
    pub(super) type_names: Vec<tree_sitter::Node<'a>>,
    pub(super) type_fields: Vec<TypeFieldCapture<'a>>,
}

/// Match-absorption context for one query pass: the extracted predicate
/// checks and the source they are validated against.
struct MatchScan<'a, 'q> {
    checks: &'q PredicateChecks,
    source_bytes: &'a [u8],
}
