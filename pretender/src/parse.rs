//! Parse pipeline for [`QueryEngine`]: turns tree-sitter query matches over
//! one source file into a [`Module`] — function captures become `CodeUnit`s,
//! metric captures feed the branch/call/assignment maps, type captures feed
//! type-definition building. Bodies moved verbatim from `engine.rs`, or
//! split along order-preserving seams, so findings stay byte-identical.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;

use crate::engine::{
    node_span, CaptureMap, FunctionCapture, QueryEngine, TypeCaptures, TypeDefCapture,
    TypeFieldCapture,
};
use crate::model::{
    CodeUnit, Diagnostic, DiagnosticSeverity, Import, Language, Module, Span, TypeDefinition,
};

impl QueryEngine {
    /// Parse `source` into a [`Module`] plus any parse diagnostics.
    pub fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        let tree = self.parse_tree(source)?;
        let root = tree.root_node();
        let lines = LineCounts::collect(root, source);
        if root.has_error() {
            let module = base_module(path, &self.lang_kind, lines);
            return Ok((module, vec![parse_error_diagnostic(path, root)]));
        }

        let source_bytes = source.as_bytes();
        let captures =
            self.collect_query_captures(root, source_bytes, CollectedCaptures::default());
        let (units, diagnostics) = self.build_units(&captures, source_bytes);
        let type_definitions = self.build_type_definitions(
            TypeCaptures {
                defs: captures.type_defs,
                names: captures.type_names,
                fields: captures.type_fields,
            },
            source_bytes,
        );
        let module = finished_module(
            base_module(path, &self.lang_kind, lines),
            units,
            captures.imports,
            type_definitions,
        );
        Ok((module, diagnostics))
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
                if capture.index == self.fn_def_idx {
                    fc.def_node = Some(capture.node);
                } else if capture.index == self.fn_name_idx {
                    fc.name_node = Some(capture.node);
                } else if capture.index == self.fn_params_idx {
                    fc.params_node = Some(capture.node);
                } else if capture.index == self.fn_body_idx {
                    fc.body_node = Some(capture.node);
                }
                fc
            })
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
        while let Some(m) = query_matches.next() {
            if m.captures.iter().any(|c| c.index == self.fn_def_idx) {
                out.functions.push(self.function_capture(m));
            } else {
                self.collect_non_function_match(m, source_bytes, &mut out);
            }
        }
        out
    }

    /// Collect a non-function match: metric captures go straight into `out`.
    fn collect_non_function_match<'a>(
        &self,
        m: &tree_sitter::QueryMatch<'_, 'a>,
        source_bytes: &'a [u8],
        out: &mut CollectedCaptures<'a>,
    ) {
        let mut pending = PendingMatch::new(self, source_bytes);
        for capture in m.captures {
            if pending.dispatch_metric(&mut out.map, capture) {
                continue;
            }
            if pending.dispatch_call(capture) {
                continue;
            }
            pending.dispatch_type(out, capture);
        }
        if let Some((id, callee)) = pending.pending_call.take() {
            out.map.calls.insert(id, callee.unwrap_or_default());
        }
        absorb_type_defs(out, &pending);
        absorb_type_fields(out, &pending);
    }

    /// Build one `CodeUnit` per function capture, pairing failures with diagnostics.
    fn build_units<'a>(
        &self,
        captures: &CollectedCaptures<'a>,
        source_bytes: &[u8],
    ) -> (Vec<CodeUnit>, Vec<Diagnostic>) {
        captures
            .functions
            .iter()
            .fold(
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

/// Accumulators filled by one pass over the query matches, in match order.
#[derive(Default)]
struct CollectedCaptures<'a> {
    functions: Vec<FunctionCapture<'a>>,
    map: CaptureMap,
    imports: Vec<Import>,
    type_defs: Vec<TypeDefCapture<'a>>,
    type_names: Vec<tree_sitter::Node<'a>>,
    type_fields: Vec<TypeFieldCapture<'a>>,
}

/// Classified line counts for one file (total, code, comment).
struct LineCounts {
    total: u32,
    code: u32,
    comment: u32,
}

impl LineCounts {
    fn collect(root: tree_sitter::Node, source: &str) -> Self {
        let lines_total = source.lines().count() as u32;
        let (code, comment) = classify_lines(root, lines_total);
        Self { total: lines_total, code, comment }
    }
}

/// Per-match slots: a pending call site plus five optional type captures.
struct PendingMatch<'a, 'e> {
    engine: &'e QueryEngine,
    source: &'a [u8],
    pending_call: Option<(usize, Option<String>)>,
    type_def_node: Option<tree_sitter::Node<'a>>,
    type_name_node: Option<tree_sitter::Node<'a>>,
    type_field_node: Option<tree_sitter::Node<'a>>,
    type_field_name_node: Option<tree_sitter::Node<'a>>,
    type_field_type_node: Option<tree_sitter::Node<'a>>,
}

impl<'a, 'e> PendingMatch<'a, 'e> {
    fn new(engine: &'e QueryEngine, source: &'a [u8]) -> Self {
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

    /// Branch/assertion/assignment captures feed the metric maps.
    fn dispatch_metric(
        &mut self,
        map: &mut CaptureMap,
        capture: &tree_sitter::QueryCapture<'a>,
    ) -> bool {
        if let Some(spec) = self.engine.branch_capture_for_index(capture.index) {
            map.branches.insert(capture.node.id(), spec);
        } else if self.engine.assert_capture_indices.contains(&capture.index) {
            map.assertions.insert(capture.node.id());
        } else if Some(capture.index) == self.engine.assign_idx {
            map.assignments.insert(capture.node.id());
        } else {
            return false;
        }
        true
    }

    /// Call and call.callee captures accumulate into the pending call site.
    fn dispatch_call(&mut self, capture: &tree_sitter::QueryCapture<'a>) -> bool {
        if Some(capture.index) == self.engine.call_idx {
            let id = capture.node.id();
            match &mut self.pending_call {
                Some((existing_id, _)) if *existing_id == id => {}
                _ => self.pending_call = Some((id, None)),
            }
        } else if Some(capture.index) == self.engine.call_callee_idx {
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
        if Some(capture.index) == self.engine.import_idx {
            if let Ok(text) = capture.node.utf8_text(self.source) {
                out.imports.push(Import {
                    module: text.to_string(),
                    name: None,
                    alias: None,
                    span: node_span(capture.node),
                });
            }
        } else if Some(capture.index) == self.engine.type_def_idx {
            self.type_def_node = Some(capture.node);
        } else if Some(capture.index) == self.engine.type_name_idx {
            self.type_name_node = Some(capture.node);
        } else if Some(capture.index) == self.engine.type_field_idx {
            self.type_field_node = Some(capture.node);
        } else if Some(capture.index) == self.engine.type_field_name_idx {
            self.type_field_name_node = Some(capture.node);
        } else if Some(capture.index) == self.engine.type_field_type_idx {
            self.type_field_type_node = Some(capture.node);
        } else {
            return false;
        }
        true
    }
}

/// Commit the match's type-definition/name captures.
fn absorb_type_defs<'a>(captures: &mut CollectedCaptures<'a>, pending: &PendingMatch<'a, '_>) {
    match pending.type_def_node {
        Some(def_node) => captures.type_defs.push(TypeDefCapture {
            node: def_node,
            name_node: pending.type_name_node,
        }),
        // A @type.name capture in a match of its own binds
        // positionally to the innermost enclosing definition
        // in build_type_definitions.
        None => {
            if let Some(name_node) = pending.type_name_node {
                captures.type_names.push(name_node);
            }
        }
    }
}

/// Commit the match's field captures, keeping the definition-owning slot.
fn absorb_type_fields<'a>(captures: &mut CollectedCaptures<'a>, pending: &PendingMatch<'a, '_>) {
    if pending.type_field_node.is_some() || pending.type_field_name_node.is_some() {
        captures.type_fields.push(TypeFieldCapture {
            node: pending
                .type_field_node
                .or(pending.type_field_name_node)
                .or(pending.type_field_type_node)
                .unwrap(),
            name_node: pending.type_field_name_node,
            type_node: pending.type_field_type_node,
            // Definition id when the same match carried
            // @type.definition; otherwise resolved by position.
            owner: pending.type_def_node.map(|n| n.id()),
        });
    }
}

/// The `Module` shell for one file; `parse` fills units/imports/types.
fn base_module(path: &Path, language: &Language, lines: LineCounts) -> Module {
    Module {
        path: path.to_path_buf(),
        language: language.clone(),
        span: Span {
            start_line: 1,
            end_line: lines.total,
        },
        lines_total: lines.total,
        lines_code: lines.code,
        lines_comment: lines.comment,
        units: Vec::new(),
        imports: Vec::new(),
        type_definitions: Vec::new(),
    }
}

/// Fill the assembled units, imports, and type definitions into the shell.
fn finished_module(
    mut module: Module,
    units: Vec<CodeUnit>,
    imports: Vec<Import>,
    type_definitions: Vec<TypeDefinition>,
) -> Module {
    module.units = units;
    module.imports = imports;
    module.type_definitions = type_definitions;
    module
}

/// The warning diagnostic emitted when the tree contains parse errors.
fn parse_error_diagnostic(path: &Path, root: tree_sitter::Node) -> Diagnostic {
    Diagnostic {
        message: format!("Parse errors detected in {}", path.display()),
        span: Some(node_span(root)),
        severity: DiagnosticSeverity::Warning,
    }
}

/// Classify each line in the file using tree-sitter node types.
/// Returns (lines_code, lines_comment).
/// A line is a comment line if every leaf node on that line is a comment.
/// Otherwise if any named non-comment leaf touches the line, it is a code line.
fn classify_lines(root: tree_sitter::Node, lines_total: u32) -> (u32, u32) {
    let (code, comment) = walk_classify(HashSet::new(), HashSet::new(), root);
    (
        code.len() as u32,
        comment.iter().filter(|&&l| l <= lines_total).count() as u32,
    )
}

/// Depth-first walk over the whole tree, classifying every leaf's lines.
fn walk_classify(
    mut code: HashSet<u32>,
    mut comment: HashSet<u32>,
    root: tree_sitter::Node,
) -> (HashSet<u32>, HashSet<u32>) {
    let mut cursor = root.walk();
    loop {
        classify_leaf(cursor.node(), &mut code, &mut comment);
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return (code, comment);
            }
        }
    }
}

/// Apply one leaf node's classification to every line it touches.
fn classify_leaf(node: tree_sitter::Node, code: &mut HashSet<u32>, comment: &mut HashSet<u32>) {
    if node.child_count() != 0 {
        return;
    }
    let start = node.start_position().row as u32 + 1;
    let end = node.end_position().row as u32 + 1;
    let is_comment = node.kind().contains("comment");
    for line in start..=end {
        if is_comment {
            if !code.contains(&line) {
                comment.insert(line);
            }
        } else if !node.kind().trim().is_empty() {
            code.insert(line);
            comment.remove(&line);
        }
    }
}
