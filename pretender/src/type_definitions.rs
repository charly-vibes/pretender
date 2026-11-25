//! Type-definition assembly from tree-sitter type captures.
//!
//! Purpose: turn the raw `@type.definition` / `@type.name` / `@type.field`
//! captures gathered during a parse into `TypeDefinition` model entries.
//!
//! # Responsibilities
//! Dedupe definition captures (first per node id), build skeletons in
//! capture order, bind split-pattern name captures to the innermost
//! enclosing definition, resolve each field's owner (in-match definition
//! id, else innermost enclosing definition), drop fields that belong to
//! a deliberately uncaptured nested type body (design D3), and attach
//! fields in (owner, start byte) order.
//!
//! # Rationale
//! Split out of `engine.rs` so the module boundary follows the
//! responsibility; `build_type_definitions` is decomposed into stage
//! helpers so every function fits the gate's limits, with bodies moved
//! verbatim to keep type-definition output ordering byte-identical.

use std::collections::HashSet;

use crate::engine::node_span;
use crate::model::{Field, Language, TypeDefinition, TypeKind};

pub(crate) struct TypeDefCapture<'a> {
    pub(crate) node: tree_sitter::Node<'a>,
    pub(crate) name_node: Option<tree_sitter::Node<'a>>,
}

pub(crate) struct TypeFieldCapture<'a> {
    pub(crate) node: tree_sitter::Node<'a>,
    pub(crate) name_node: Option<tree_sitter::Node<'a>>,
    pub(crate) type_node: Option<tree_sitter::Node<'a>>,
    /// Definition node id when the same match carried @type.definition.
    pub(crate) owner: Option<usize>,
}

/// Raw type captures gathered during a parse, passed as one bundle to
/// `build_type_definitions`.
pub(crate) struct TypeCaptures<'a> {
    pub(crate) defs: Vec<TypeDefCapture<'a>>,
    pub(crate) names: Vec<tree_sitter::Node<'a>>,
    pub(crate) fields: Vec<TypeFieldCapture<'a>>,
}

/// Assemble `TypeDefinition`s from raw type captures. Fields whose
/// match carried no `@type.definition` bind to the innermost enclosing
/// definition; captures with no enclosing definition are dropped.
/// Fields keep source order.
pub(crate) fn build_type_definitions(
    captures: TypeCaptures,
    source: &[u8],
    language: Language,
) -> Vec<TypeDefinition> {
    let TypeCaptures {
        defs,
        names,
        fields,
    } = captures;
    // The same definition node can be captured by several patterns;
    // keep the first capture per node id.
    let (defs, seen) = dedupe_defs(defs);
    let mut built = build_skeleton(&defs, source, language);
    bind_split_names(&names, &defs, &mut built, source);
    let owned = owned_fields(fields, &defs, &seen, source);
    attach_fields(&mut built, owned);
    built
}

/// Keep the first capture per definition node id; the resulting id set
/// also records which type bodies were deliberately captured.
fn dedupe_defs(defs: Vec<TypeDefCapture>) -> (Vec<TypeDefCapture>, HashSet<usize>) {
    let mut seen = HashSet::new();
    let defs = defs
        .into_iter()
        .filter(|def| seen.insert(def.node.id()))
        .collect();
    (defs, seen)
}

/// Build one empty `TypeDefinition` per deduped definition capture, in
/// capture order, carrying in-match names and best-effort kinds.
fn build_skeleton(
    defs: &[TypeDefCapture],
    source: &[u8],
    language: Language,
) -> Vec<TypeDefinition> {
    defs.iter()
        .map(|def| TypeDefinition {
            name: def
                .name_node
                .and_then(|n| n.utf8_text(source).ok())
                .unwrap_or_default()
                .to_string(),
            kind: determine_type_kind(def.node.kind()),
            span: node_span(def.node),
            fields: Vec::new(),
            language: language.clone(),
        })
        .collect()
}

/// Bind split-pattern `@type.name` captures: fill only definitions whose
/// in-match name capture was absent, innermost-enclosing definition first.
fn bind_split_names(
    names: &[tree_sitter::Node],
    defs: &[TypeDefCapture],
    built: &mut [TypeDefinition],
    source: &[u8],
) {
    for name_node in names {
        fill_split_name(*name_node, defs, built, source);
    }
}

/// Bind one split-pattern `@type.name` capture to the innermost enclosing
/// definition, filling only definitions whose in-match name was absent.
fn fill_split_name(
    name_node: tree_sitter::Node,
    defs: &[TypeDefCapture],
    built: &mut [TypeDefinition],
    source: &[u8],
) {
    if let Some(idx) = innermost_enclosing_type(defs, name_node) {
        if built[idx].name.is_empty() {
            fill_missing_name(&mut built[idx], name_node, source);
        }
    }
}

fn fill_missing_name(def: &mut TypeDefinition, name_node: tree_sitter::Node, source: &[u8]) {
    if let Ok(text) = name_node.utf8_text(source) {
        def.name = text.to_string();
    }
}

/// Resolve each field capture to its owning definition, in capture order.
fn owned_fields(
    fields: Vec<TypeFieldCapture>,
    defs: &[TypeDefCapture],
    captured_def_ids: &HashSet<usize>,
    source: &[u8],
) -> Vec<(usize, usize, Field)> {
    fields
        .into_iter()
        .filter_map(|field| field_entry(field, defs, captured_def_ids, source))
        .collect()
}

/// The (owner, start byte, field) entry for one captured member, or
/// None when it carries no name, binds to no definition, or belongs
/// to a deliberately uncaptured nested type body.
fn field_entry(
    field: TypeFieldCapture,
    defs: &[TypeDefCapture],
    captured_def_ids: &HashSet<usize>,
    source: &[u8],
) -> Option<(usize, usize, Field)> {
    // The model contract requires each captured field to carry a
    // name; entries without a name capture are not fields.
    let name_node = field.name_node?;
    let name = name_node.utf8_text(source).ok()?;
    let owner = resolve_field_owner(&field, defs)?;
    // Members of a deliberately uncaptured nested type must not
    // be attributed to the enclosing captured definition.
    if field_inside_uncaptured_type(field.node, defs[owner].node, captured_def_ids) {
        return None;
    }
    let entry = captured_field(&field, name, source);
    Some((owner, field.node.start_byte(), entry))
}

/// Build the `Field` model entry for one captured type member.
fn captured_field(field: &TypeFieldCapture, name: &str, source: &[u8]) -> Field {
    Field {
        name: name.to_string(),
        type_name: field
            .type_node
            .and_then(|t| t.utf8_text(source).ok())
            .map(str::to_string),
        span: node_span(field.node),
    }
}

/// Resolve a field's owning definition: the in-match `@type.definition`
/// node id when present, else the innermost enclosing definition.
fn resolve_field_owner(field: &TypeFieldCapture, defs: &[TypeDefCapture]) -> Option<usize> {
    field
        .owner
        .and_then(|id| defs.iter().position(|def| def.node.id() == id))
        .or_else(|| innermost_enclosing_type(defs, field.node))
}

/// Sort entries by (owner, start byte) — a stable sort, so same-start
/// captures keep capture order — and attach each field to its definition.
fn attach_fields(built: &mut [TypeDefinition], mut owned: Vec<(usize, usize, Field)>) {
    owned.sort_by_key(|(owner, start, _)| (*owner, *start));
    for (owner, _, field) in owned {
        built[owner].fields.push(field);
    }
}

/// Best-effort kind mapping (D3): struct declarations → Struct, record
/// declarations → Record, any other captured type node → Class.
fn determine_type_kind(node_kind: &str) -> TypeKind {
    match node_kind {
        "struct_item" | "struct_declaration" | "struct_specifier" | "struct_definition" => {
            TypeKind::Struct
        }
        "record_declaration" | "record_definition" | "defrecord" => TypeKind::Record,
        other => {
            if other.contains("record") {
                TypeKind::Record
            } else if other.contains("struct") {
                TypeKind::Struct
            } else {
                TypeKind::Class
            }
        }
    }
}

/// Index of the smallest captured definition whose byte range encloses
/// `node` — the innermost enclosing @type.definition.
fn innermost_enclosing_type<'a>(
    defs: &[TypeDefCapture<'a>],
    node: tree_sitter::Node<'a>,
) -> Option<usize> {
    defs.iter()
        .enumerate()
        .filter(|(_, def)| {
            def.node.start_byte() <= node.start_byte() && node.end_byte() <= def.node.end_byte()
        })
        .max_by_key(|(_, def)| {
            (
                def.node.start_byte(),
                std::cmp::Reverse(def.node.end_byte()),
            )
        })
        .map(|(idx, _)| idx)
}

/// True when the node kind introduces a type-definition body.
/// Grammar-agnostic name matching, mirroring `determine_type_kind`;
/// bare body/list wrappers are skipped because a definition capture
/// anchors on the definition node itself, not on its body child.
fn is_type_body_kind(kind: &str) -> bool {
    if kind.ends_with("_body") || kind.ends_with("_list") {
        return false;
    }
    [
        "struct",
        "union",
        "enum",
        "class",
        "interface",
        "record",
        "trait",
    ]
    .iter()
    .any(|word| kind.contains(word))
}

/// True when `field_node` sits inside a type body that is neither the
/// bound definition nor a captured definition itself — the member
/// belongs to that deliberately uncaptured inner type (design D3),
/// e.g. C/C++ anonymous unions/structs or Go inline struct types.
fn field_inside_uncaptured_type(
    field_node: tree_sitter::Node,
    owner: tree_sitter::Node,
    captured_def_ids: &HashSet<usize>,
) -> bool {
    let mut current = field_node.parent();
    while let Some(node) = current {
        if node.id() == owner.id() {
            return false;
        }
        if is_type_body_kind(node.kind()) && !captured_def_ids.contains(&node.id()) {
            return true;
        }
        current = node.parent();
    }
    false
}
