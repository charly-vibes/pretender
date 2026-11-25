//! Mutability analysis: void-mutator detection and mutable-state ratio.
//!
//! Shared between the void-mutator and mutable-state check proposals.

use crate::model::{CodeUnit, Language, UnitKind};
use std::path::Path;

/// Result of mutable-binding analysis for a file.
#[derive(Debug, Clone, Default)]
pub struct MutableBindingAnalysis {
    pub mutable_count: u32,
    pub total_count: u32,
}

impl MutableBindingAnalysis {
    pub fn ratio(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.mutable_count as f64 / self.total_count as f64
        }
    }
}

/// Count mutable vs total bindings in source code.
///
/// Per-language patterns:
/// - Rust: `let mut` counts as mutable, `let` counts as total
/// - JavaScript/TypeScript: `let`/`var` counts as mutable, `const` counts as total
/// - Java: non-final local variable declarations
/// - C++: `mutable` keyword, non-const variable declarations
/// - Python: excluded (no `const` keyword)
pub fn count_mutable_bindings(source: &str, language: &Language) -> MutableBindingAnalysis {
    match language {
        Language::Rust => count_rust_mutable_bindings(source),
        Language::JavaScript | Language::TypeScript => count_js_mutable_bindings(source),
        Language::Java => count_java_mutable_bindings(source),
        Language::C | Language::Cpp => count_cpp_mutable_bindings(source),
        _ => MutableBindingAnalysis::default(),
    }
}

fn count_rust_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    let mutable = source
        .lines()
        .filter(|line| line.trim().starts_with("let mut "))
        .count() as u32;
    let total = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("let ") || trimmed.starts_with("let(")
        })
        .count() as u32;
    MutableBindingAnalysis {
        mutable_count: mutable,
        total_count: total,
    }
}

fn count_js_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    let mutable = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("let ") || trimmed.starts_with("var ")
        })
        .count() as u32;
    let total = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("let ")
                || trimmed.starts_with("var ")
                || trimmed.starts_with("const ")
        })
        .count() as u32;
    MutableBindingAnalysis {
        mutable_count: mutable,
        total_count: total,
    }
}

/// Java type keywords that mark a local variable declaration line.
fn java_declares_type(trimmed: &str) -> bool {
    trimmed.contains("String ")
        || trimmed.contains("int ")
        || trimmed.contains("boolean ")
        || trimmed.contains("long ")
        || trimmed.contains("double ")
}

fn count_java_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    // Count non-final local variable declarations.
    // Match type name followed by variable name (simplified)
    let total = source
        .lines()
        .filter(|line| java_declares_type(line.trim()))
        .count() as u32;
    let mutable = source
        .lines()
        .filter(|line| java_declares_type(line.trim()) && !line.trim().contains("final "))
        .count() as u32;
    MutableBindingAnalysis {
        mutable_count: mutable,
        total_count: total,
    }
}

fn count_cpp_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    // Flag `mutable` keyword declarations
    let mutable_decls = source
        .lines()
        .filter(|line| line.trim().contains("mutable "))
        .count() as u32;
    // Count auto/type declarations that aren't const
    let type_decls = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            (trimmed.starts_with("auto ")
                || trimmed.contains("int ")
                || trimmed.contains("string "))
                && !trimmed.starts_with("const ")
                && !trimmed.contains(" const ")
        })
        .count() as u32;
    let total = mutable_decls + type_decls;
    MutableBindingAnalysis {
        mutable_count: total,
        total_count: total,
    }
}

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
    let void_keywords = match void_keywords_for(language) {
        Some(keywords) => keywords,
        None => return Vec::new(),
    };

    let lines: Vec<&str> = source.lines().collect();
    let mut result = Vec::new();

    for unit in units {
        if let Some(mutator) = void_mutator_for_unit(unit, language, &lines, void_keywords) {
            result.push(mutator);
        }
    }

    canonical_mutator_order(result)
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

    // Check if void/unit return type
    if !is_void_return(&body_text, language, void_keywords) {
        return None;
    }

    // Check for direct field assignment
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
        Language::Python => {
            // Check for no return statement or bare `return`
            let has_return = body_text.contains("return ");
            let has_value_return = regex_check(body_text, r"return\s+\w");
            !has_return || !has_value_return
        }
        Language::Java | Language::Cpp | Language::CSharp => {
            // Check if the method signature contains `void`
            let first_line = body_text.lines().next().unwrap_or("");
            first_line.trim().starts_with("void ") || first_line.contains("void ")
        }
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

/// Simple regex-like check (no actual regex — just string matching).
fn regex_check(text: &str, pattern: &str) -> bool {
    // Simplified: just check if the pattern's key parts exist
    match pattern {
        r"return\s+\w" => {
            // Check for "return <word>" pattern
            let lines: Vec<&str> = text.lines().collect();
            for line in lines {
                let trimmed = line.trim();
                if trimmed.starts_with("return ") && trimmed.len() > 7 {
                    let after = trimmed[7..].trim();
                    if !after.is_empty() && !after.starts_with('\'') && !after.starts_with('"') {
                        return true;
                    }
                }
            }
            false
        }
        _ => text.contains(pattern),
    }
}
