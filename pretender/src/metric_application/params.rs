//! Parameter extraction for `CodeUnit` assembly.
//!
//! Purpose: turn a captured function's parameter-list node into `Parameter`
//! entries across grammars (named, typed, pattern, splat, and Clojure
//! binding forms).
//! Responsibilities: per-node-kind parameter naming, type extraction.
//! Rationale: split out of `metric_application.rs` along the assembly /
//! parameter-parsing seam; extraction order and naming stay identical.

use super::FunctionCapture;
use crate::engine::node_span;
use crate::model::Parameter;

pub(super) fn unit_parameters(fc: &FunctionCapture, source: &[u8]) -> Vec<Parameter> {
    fc.params_node
        .map(|p| extract_params(p, source))
        .unwrap_or_default()
}

fn extract_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<Parameter> {
    (0..params_node.child_count())
        .filter_map(|i| {
            let child = params_node.child(i)?;
            param_name(&child, source).map(|name| param_entry(&child, name, source))
        })
        .collect()
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
        // Clojure binding forms (tree-sitter-clojure): bare `sym_lit`
        // parameters, the `&` variadic marker, and destructuring
        // `vec_lit`/`map_lit` bindings. Scoped by kind: only adapters
        // whose `@function.parameters` capture yields these kinds are
        // affected (the pinned grammar emits them only in Clojure).
        "sym_lit" | "vec_lit" | "map_lit" => clojure_param_name(child, source),
        _ => None,
    }
}

/// Clojure binding names: a bare `sym_lit` is one parameter as written;
/// `&` marks the variadic boundary and is itself not a parameter (the
/// binding after it counts once, like Python's `*rest`); a destructuring
/// `vec_lit`/`map_lit` binding counts as one parameter named by its
/// source text — per-element granularity is declared unsupported
/// (languages/clojure/README.md).
fn clojure_param_name(child: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    let text = child.utf8_text(source).ok()?;
    if text == "&" {
        None
    } else {
        Some(text.to_string())
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
    (0..pattern.child_count()).find_map(|i| {
        let n = pattern.child(i)?;
        if n.kind() == "identifier" {
            n.utf8_text(source).ok().map(str::to_string)
        } else {
            None
        }
    })
}

/// Splat parameters keep their `*`/`**` prefix before the identifier.
fn splat_param_name(child: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let prefix = if child.kind() == "list_splat_pattern" {
        "*"
    } else {
        "**"
    };
    let ident = (0..child.child_count()).find_map(|i| {
        let n = child.child(i)?;
        if n.kind() == "identifier" {
            n.utf8_text(source).ok()
        } else {
            None
        }
    })?;
    Some(format!("{prefix}{ident}"))
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
