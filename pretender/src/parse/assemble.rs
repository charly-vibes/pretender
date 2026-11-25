//! Match absorption and module assembly for the parse pipeline.
//!
//! # Responsibilities
//! Commit a finished match's type captures into the pass accumulators
//! (`absorb_*`), build the type definitions from them, and assemble the
//! `Module` shell (`base_module`, `finished_module`) plus the parse-error
//! diagnostic.
//!
//! # Rationale
//! Split from the parse pipeline (pretender-5zo.15.12); bodies moved
//! verbatim so findings stay byte-identical.
use super::pending::PendingMatch;
use super::CollectedCaptures;
use crate::engine::node_span;
use crate::line_counts::LineCounts;
use crate::model::{
    CodeUnit, Diagnostic, DiagnosticSeverity, Import, Language, Module, Span, TypeDefinition,
};
use crate::type_definitions::{
    build_type_definitions, TypeCaptures, TypeDefCapture, TypeFieldCapture,
};

/// Commit the match's type-definition/name captures.
pub(super) fn absorb_type_defs<'a>(
    captures: &mut CollectedCaptures<'a>,
    pending: &PendingMatch<'a, '_>,
) {
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
pub(super) fn absorb_type_fields<'a>(
    captures: &mut CollectedCaptures<'a>,
    pending: &PendingMatch<'a, '_>,
) {
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

/// Build the type definitions from a pass's type captures, returning the
/// drained remainder (functions, map, and imports stay untouched).
pub(super) fn build_type_captures<'a>(
    mut captures: CollectedCaptures<'a>,
    source_bytes: &[u8],
    lang_kind: &Language,
) -> (Vec<TypeDefinition>, CollectedCaptures<'a>) {
    let definitions = build_type_definitions(
        TypeCaptures {
            defs: std::mem::take(&mut captures.type_defs),
            names: std::mem::take(&mut captures.type_names),
            fields: std::mem::take(&mut captures.type_fields),
        },
        source_bytes,
        lang_kind.clone(),
    );
    (definitions, captures)
}

/// The `Module` shell for one file; `parse` fills units/imports/types.
pub(super) fn base_module(
    path: &std::path::Path,
    language: &Language,
    lines: LineCounts,
) -> Module {
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
pub(super) fn finished_module(
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
pub(super) fn parse_error_diagnostic(
    path: &std::path::Path,
    root: tree_sitter::Node,
) -> Diagnostic {
    Diagnostic {
        message: format!("Parse errors detected in {}", path.display()),
        span: Some(node_span(root)),
        severity: DiagnosticSeverity::Warning,
    }
}
