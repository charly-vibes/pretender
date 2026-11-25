//! Mutable-binding counting per language.
//!
//! Purpose: count mutable vs total bindings in source to feed the
//! mutable-state ratio check.
//! Responsibilities: per-language line heuristics for binding keywords.
//! Rationale: split out of `mutability_metrics.rs` along the bindings /
//! void-mutator seam; counts stay identical.

use crate::model::Language;

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

/// JS/TS `let`/`var` declarations count as mutable bindings.
fn js_is_mutable(trimmed: &str) -> bool {
    trimmed.starts_with("let ") || trimmed.starts_with("var ")
}

/// Any JS/TS binding keyword (`let`/`var`/`const`) counts as a binding.
fn js_counts_as_binding(trimmed: &str) -> bool {
    js_is_mutable(trimmed) || trimmed.starts_with("const ")
}

fn count_js_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    let mutable = source
        .lines()
        .filter(|line| js_is_mutable(line.trim()))
        .count() as u32;
    let total = source
        .lines()
        .filter(|line| js_counts_as_binding(line.trim()))
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

/// C++ type declarations that count toward bindings when not `const`.
fn cpp_declares_mutable_type(trimmed: &str) -> bool {
    let is_typed =
        trimmed.starts_with("auto ") || trimmed.contains("int ") || trimmed.contains("string ");
    is_typed && !trimmed.starts_with("const ") && !trimmed.contains(" const ")
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
        .filter(|line| cpp_declares_mutable_type(line.trim()))
        .count() as u32;
    let total = mutable_decls + type_decls;
    MutableBindingAnalysis {
        mutable_count: total,
        total_count: total,
    }
}
