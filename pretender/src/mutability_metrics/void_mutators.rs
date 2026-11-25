//! Void-mutator detection.
//!
//! Purpose: detect void-return methods that directly assign to
//! `self`/`this` fields, feeding the void-mutator check.
//! Responsibilities: language void-keyword dispatch, unit eligibility,
//! body slicing, return-type heuristics.
//! Rationale: split out of `mutability_metrics.rs` along the bindings /
//! void-mutator seam; detection semantics stay identical, and the result
//! stays canonicalized so caller iteration order cannot leak into findings.

use crate::model::{CodeUnit, Language, UnitKind};
use std::path::Path;

/// A void-return method that mutates `self`/`this` fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidMutator {
    #[allow(dead_code)] // informative metadata for future output formats
    pub name: String,
    #[allow(dead_code)] // informative metadata for future output formats
    pub line: u32,
}

/// Detect void-return methods that directly assign to `self`/`this` fields.
///
/// A void mutator is a method that:
/// 1. Returns void/unit/None (no return value)
/// 2. Contains direct field assignment to `self.`/`this.` fields
/// 3. Is NOT a constructor or initializer
///
/// Transitive mutation via child method calls (`self.list.add(x)`) is NOT
/// counted — only direct field assignment (`self.field = value`).
pub fn detect_void_mutators(
    source: &str,
    language: &Language,
    units: &[CodeUnit],
    _path: &Path,
) -> Vec<VoidMutator> {
    let Some(void_keywords) = void_keywords_for(language) else {
        return Vec::new();
    };

    let lines: Vec<&str> = source.lines().collect();
    let mutators = units
        .iter()
        .filter_map(|unit| void_mutator_for_unit(unit, language, &lines, void_keywords))
        .collect();

    canonical_mutator_order(mutators)
}

/// Void-indicator keywords per language; `None` when the language has no
/// void-mutator detection.
fn void_keywords_for(language: &Language) -> Option<&'static [&'static str]> {
    match language {
        Language::Rust => Some(&["fn "][..]),
        Language::Python => Some(&["def "][..]),
        Language::JavaScript | Language::TypeScript => Some(&["function ", "method", "=> {"][..]),
        Language::Java | Language::C | Language::Cpp | Language::CSharp => Some(&["void "][..]),
        Language::Go => Some(&["func "][..]),
        _ => None,
    }
}

/// Decide whether one unit is a void mutator, or `None` if it is not.
fn void_mutator_for_unit(
    unit: &CodeUnit,
    language: &Language,
    lines: &[&str],
    void_keywords: &[&str],
) -> Option<VoidMutator> {
    // Skip constructors, initializers, and non-method kinds
    if matches!(unit.kind, UnitKind::Ctor | UnitKind::Initializer) {
        return None;
    }

    // Only consider methods (not standalone functions)
    if !is_method_unit(unit, language) {
        return None;
    }

    void_mutator_in_body(unit, language, lines, void_keywords)
}

/// Check one method's body for a void return and direct field assignment.
fn void_mutator_in_body(
    unit: &CodeUnit,
    language: &Language,
    lines: &[&str],
    void_keywords: &[&str],
) -> Option<VoidMutator> {
    let body_text = unit_body_text(unit, lines)?;
    if !is_void_return(&body_text, language, void_keywords) {
        return None;
    }
    if !has_direct_field_assignment(&body_text, language) {
        return None;
    }

    Some(VoidMutator {
        name: unit.name.clone(),
        line: unit.span.start_line,
    })
}

/// Slice a unit's body lines out of `lines`; `None` when out of bounds.
fn unit_body_text(unit: &CodeUnit, lines: &[&str]) -> Option<String> {
    let start = unit.span.start_line as usize;
    let end = unit.span.end_line as usize;
    if start <= end && end <= lines.len() {
        Some(lines[start - 1..end].join("\n"))
    } else {
        None
    }
}

/// Check for direct field assignment (`self.`/`this.` fields) in a body.
fn has_direct_field_assignment(body_text: &str, language: &Language) -> bool {
    match language {
        Language::Rust | Language::Python => {
            body_text.contains("self.") && body_text.contains(" = ")
        }
        Language::Java
        | Language::Cpp
        | Language::CSharp
        | Language::JavaScript
        | Language::TypeScript => body_text.contains("this.") && body_text.contains(" = "),
        _ => false,
    }
}

/// Whether a unit counts as a method (not a standalone function).
fn is_method_unit(unit: &CodeUnit, language: &Language) -> bool {
    match language {
        Language::Rust => unit.name.contains("self") || has_self_param(&unit.parameters),
        Language::Python => unit.name.contains("self") || has_self_param(&unit.parameters),
        Language::Java
        | Language::Cpp
        | Language::CSharp
        | Language::JavaScript
        | Language::TypeScript => true, // Assume all class members are methods
        _ => false,
    }
}

/// pretender-gv5: the caller's unit iteration order (walk order,
/// parallel collection) must not leak into the result. Canonicalize by
/// stable keys so repeated scans of an unchanged tree produce an
/// identical finding vector; the finding SET is unchanged.
fn canonical_mutator_order(mut result: Vec<VoidMutator>) -> Vec<VoidMutator> {
    result.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));
    result
}

/// Check if a parameter list contains `self` or `&self`.
fn has_self_param(params: &[crate::model::Parameter]) -> bool {
    params.iter().any(|p| {
        let name = p.name.trim();
        name == "self" || name == "&self" || name == "mut self" || name == "&mut self"
    })
}

/// Heuristic check for void return type based on language syntax.
fn is_void_return(body_text: &str, language: &Language, _void_keywords: &[&str]) -> bool {
    match language {
        Language::Rust => {
            // Look for fn signature with no return type arrow
            !body_text.contains("-> ")
        }
        Language::Python => !python_has_value_return(body_text),
        Language::Java | Language::Cpp | Language::CSharp => java_like_is_void(body_text),
        Language::JavaScript | Language::TypeScript => {
            // Check for no return statement
            !body_text.contains("return ")
        }
        Language::Go => {
            // Check for no return type
            !body_text.contains(") ") && !body_text.contains("(")
        }
        _ => false,
    }
}

/// Python: a value return is `return <expr>` (not a bare `return`).
fn python_has_value_return(body_text: &str) -> bool {
    body_text.contains("return ") && regex_check(body_text, r"return\s+\w")
}

/// Java family: void when the first body line declares a `void` type.
fn java_like_is_void(body_text: &str) -> bool {
    let first_line = body_text.lines().next().unwrap_or("");
    first_line.contains("void ")
}

/// Simple regex-like check (no actual regex — just string matching).
fn regex_check(text: &str, pattern: &str) -> bool {
    // Simplified: just check if the pattern's key parts exist
    match pattern {
        r"return\s+\w" => text.lines().any(is_value_return_line),
        _ => text.contains(pattern),
    }
}

/// True when a trimmed line matches `return <unquoted value>`.
fn is_value_return_line(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with("return ") || trimmed.len() <= 7 {
        return false;
    }
    is_unquoted_word(&trimmed[7..])
}

/// The text after `return ` counts when it holds a non-empty, unquoted word.
fn is_unquoted_word(after: &str) -> bool {
    let after = after.trim();
    !after.is_empty() && !after.starts_with('\'') && !after.starts_with('"')
}
