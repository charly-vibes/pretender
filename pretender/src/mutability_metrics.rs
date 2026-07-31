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
    let mut mutable = 0u32;
    let mut total = 0u32;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("let ") || trimmed.starts_with("let(") {
            total += 1;
            if trimmed.starts_with("let mut ") {
                mutable += 1;
            }
        }
    }
    MutableBindingAnalysis {
        mutable_count: mutable,
        total_count: total,
    }
}

fn count_js_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    let mut mutable = 0u32;
    let mut total = 0u32;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("let ") || trimmed.starts_with("var ") {
            mutable += 1;
            total += 1;
        } else if trimmed.starts_with("const ") {
            total += 1;
        }
    }
    MutableBindingAnalysis {
        mutable_count: mutable,
        total_count: total,
    }
}

fn count_java_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    // Count non-final local variable declarations
    let mut mutable = 0u32;
    let mut total = 0u32;
    for line in source.lines() {
        let trimmed = line.trim();
        // Match type name followed by variable name (simplified)
        if trimmed.contains("String ")
            || trimmed.contains("int ")
            || trimmed.contains("boolean ")
            || trimmed.contains("long ")
            || trimmed.contains("double ")
        {
            if !trimmed.starts_with("final ") && !trimmed.contains("final ") {
                mutable += 1;
            }
            total += 1;
        }
    }
    MutableBindingAnalysis {
        mutable_count: mutable,
        total_count: total,
    }
}

fn count_cpp_mutable_bindings(source: &str) -> MutableBindingAnalysis {
    let mut mutable = 0u32;
    let mut total = 0u32;
    for line in source.lines() {
        let trimmed = line.trim();
        // Flag `mutable` keyword declarations
        if trimmed.contains("mutable ") {
            mutable += 1;
            total += 1;
        }
        // Count auto/type declarations that aren't const
        if (trimmed.starts_with("auto ") || trimmed.contains("int ") || trimmed.contains("string "))
            && !trimmed.starts_with("const ")
            && !trimmed.contains(" const ")
        {
            mutable += 1;
            total += 1;
        }
    }
    MutableBindingAnalysis {
        mutable_count: mutable,
        total_count: total,
    }
}

/// A void-return method that mutates `self`/`this` fields.
#[derive(Debug, Clone)]
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
    let void_keywords = match language {
        Language::Rust => &["fn "][..],
        Language::Python => &["def "][..],
        Language::JavaScript | Language::TypeScript => &["function ", "method", "=> {"][..],
        Language::Java | Language::C | Language::Cpp | Language::CSharp => &["void "][..],
        Language::Go => &["func "][..],
        _ => return Vec::new(),
    };

    let mut result = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for unit in units {
        // Skip constructors, initializers, and non-method kinds
        if matches!(unit.kind, UnitKind::Ctor | UnitKind::Initializer) {
            continue;
        }

        // Only consider methods (not standalone functions)
        let is_method = match language {
            Language::Rust => unit.name.contains("self") || has_self_param(&unit.parameters),
            Language::Python => unit.name.contains("self") || has_self_param(&unit.parameters),
            Language::Java | Language::Cpp | Language::CSharp | Language::JavaScript | Language::TypeScript => true, // Assume all class members are methods
            _ => false,
        };

        if !is_method {
            continue;
        }

        // Check if the method body is void
        let start = unit.span.start_line as usize;
        let end = unit.span.end_line as usize;
        let body_lines = if start <= end && end <= lines.len() {
            &lines[start - 1..end]
        } else {
            continue;
        };

        let body_text = body_lines.join("\n");

        // Check if void/unit return type
        let is_void = is_void_return(&body_text, language, void_keywords);

        if !is_void {
            continue;
        }

        // Check for direct field assignment
        let has_direct_mutation = match language {
            Language::Rust | Language::Python => {
                body_text.contains("self.") && body_text.contains(" = ")
            }
            Language::Java | Language::Cpp | Language::CSharp | Language::JavaScript | Language::TypeScript => {
                body_text.contains("this.") && body_text.contains(" = ")
            }
            _ => false,
        };

        if has_direct_mutation {
            result.push(VoidMutator {
                name: unit.name.clone(),
                line: unit.span.start_line,
            });
        }
    }

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
            first_line.trim().starts_with("void ")
                || first_line.contains("void ")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Parameter, Span};

    fn make_unit(name: &str, kind: UnitKind, start: u32, end: u32, params: Vec<&str>) -> CodeUnit {
        CodeUnit {
            name: name.to_string(),
            kind,
            span: Span { start_line: start, end_line: end },
            parameters: params.into_iter().map(|p| Parameter {
                name: p.to_string(),
                span: Span { start_line: start, end_line: start },
                type_name: None,
            }).collect(),
            body: crate::model::Block {
                span: Span { start_line: start, end_line: end },
                nesting: 0,
                children: vec![],
            },
            is_exported: false,
            assertions: 0,
            parent_class: None,
        }
    }

    #[test]
    fn detects_rust_void_mutator() {
        let source = r#"
impl MyStruct {
    fn update(&mut self, value: i32) {
        self.field = value;
    }
}
"#;
        let units = vec![make_unit("update", UnitKind::Method, 3, 5, vec!["&mut self", "value"])];
        let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "update");
    }

    #[test]
    fn pure_function_not_flagged() {
        let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let units = vec![make_unit("add", UnitKind::Function, 1, 3, vec!["a", "b"])];
        let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn transitive_mutation_not_flagged() {
        let source = r#"
impl MyStruct {
    fn update(&mut self) {
        self.list.add(42);  // method call, not direct assignment
    }
}
"#;
        let units = vec![make_unit("update", UnitKind::Method, 2, 4, vec!["&mut self"])];
        let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
        // self.list.add(42) is a method call, not self.field = value
        // The body has "self." but not " = " — so it should NOT be flagged
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn constructor_not_flagged() {
        let source = r#"
impl MyStruct {
    fn new() -> Self {
        Self { field: 42 }
    }
}
"#;
        let units = vec![make_unit("new", UnitKind::Ctor, 2, 4, vec![])];
        let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn unsupported_language_returns_empty() {
        let source = "nothing";
        let units = vec![];
        let result = detect_void_mutators(source, &Language::Julia, &units, Path::new("test.jl"));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn rust_mutable_ratio_half() {
        let source = r#"
let x = 1;
let mut y = 2;
let z = 3;
let mut w = 4;
"#;
        let analysis = count_mutable_bindings(source, &Language::Rust);
        assert_eq!(analysis.total_count, 4);
        assert_eq!(analysis.mutable_count, 2);
        assert!((analysis.ratio() - 0.5).abs() < 0.001);
    }

    #[test]
    fn rust_no_mutable_bindings() {
        let source = r#"let x = 1;"#;
        let analysis = count_mutable_bindings(source, &Language::Rust);
        assert_eq!(analysis.total_count, 1);
        assert_eq!(analysis.mutable_count, 0);
        assert_eq!(analysis.ratio(), 0.0);
    }

    #[test]
    fn js_tracks_let_var_const() {
        let source = r#"
const x = 1;
let y = 2;
var z = 3;
"#;
        let analysis = count_mutable_bindings(source, &Language::JavaScript);
        assert_eq!(analysis.total_count, 3);
        assert_eq!(analysis.mutable_count, 2);
    }

    #[test]
    fn python_returns_zero() {
        let source = "x = 1\ny = 2";
        let analysis = count_mutable_bindings(source, &Language::Python);
        assert_eq!(analysis.total_count, 0);
        assert_eq!(analysis.mutable_count, 0);
    }

    #[test]
    fn empty_source_zero_ratio() {
        let analysis = count_mutable_bindings("", &Language::Rust);
        assert_eq!(analysis.ratio(), 0.0);
    }
}